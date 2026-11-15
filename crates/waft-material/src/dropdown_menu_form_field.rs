//! Flutter counterpart: material_ui's `dropdown_menu_form_field.dart`.
//!
//! [`DropdownMenuFormField`] is a `FormField<T>` leaf: its widget carries the form field's
//! [`FormFieldData`] and the arguments Dart's constructor captures in the builder closure, kept
//! as the [`DropdownMenu`] the builder builds; [`DropdownMenuFormField::build_field`] is that
//! closure.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Offset, TextEditingValue, TextInputAction, TextInputType};
use waft_foundation::{App, Handle, ValueChanged};
use waft_painting::{EdgeInsets, EdgeInsetsGeometry, TextAlign, TextStyle};
use waft_services::{RestorationBucket, TextInputFormatterRef};
use waft_widgets::{
    AnyFormFieldState, AnyMenuController, AnyTextEditingController, AutovalidateMode, BuildContext,
    FocusNode, FormFieldData, FormFieldLeaf, FormFieldState, FormFieldStateData,
    FormFieldStateLeaf, IntoWidget, KeyRef, RestorableListenable, RestorableProperty,
    RestorableTextEditingController, RestorationMixin, RestorationMixinData, State, StateData,
    StatefulWidget, TextEditingController, TextEditingControllerLeaf, UnmanagedRestorationScope,
    WidgetRef,
};

use crate::dropdown_menu::{
    DropdownMenu, DropdownMenuCloseBehavior, DropdownMenuDecorationBuilder, DropdownMenuEntry,
    FilterCallback, SearchCallback,
};
use crate::input_decorator::{InputDecoration, InputDecorationThemeData};
use crate::menu_style::MenuStyle;

/// A [`DropdownMenu`] that contains a `TextField`.
///
/// This is a convenience widget that wraps a [`DropdownMenu`] widget in a `FormField`.
///
/// A `Form` ancestor is not required. The `Form` allows one to save, reset, or validate
/// multiple fields at once. To use without a `Form`, pass a `GlobalKey` to the constructor and
/// use `GlobalKey::current_state` to save or reset the form field.
///
/// The `value` parameter maps to [`FormFieldData::initial_value`].
///
/// See also:
///
///  * [`DropdownMenu`], which is the underlying text field without the `Form` integration.
pub struct DropdownMenuFormField<T: Clone + PartialEq + 'static> {
    form_field: FormFieldData<T>,

    /// Called when the user selects an item.
    pub on_selected: Option<ValueChanged<Option<T>>>,

    /// Controls the text being edited or selected in the menu.
    ///
    /// If null, this widget will create its own `TextEditingController`.
    pub controller: Option<AnyTextEditingController>,

    /// Descriptions of the menu items in the [`DropdownMenuFormField`].
    ///
    /// This is a required parameter. It is recommended that at least one [`DropdownMenuEntry`]
    /// is provided. If this is an empty list, the menu will be empty and only contain space for
    /// padding.
    pub dropdown_menu_entries: Rc<[DropdownMenuEntry<T>]>,

    /// The constructor's other arguments, which Dart's builder closure captures: the
    /// [`DropdownMenu`] it builds before the state fills the controller, the selection and the
    /// decoration.
    dropdown_menu: DropdownMenu<T>,
    label: Option<WidgetRef>,
    hint_text: Option<String>,
    helper_text: Option<String>,
    decoration_builder: Option<DropdownMenuDecorationBuilder>,
}

impl<T: Clone + PartialEq + 'static> DropdownMenuFormField<T> {
    /// Creates a [`DropdownMenu`] widget that is a `FormField`.
    ///
    /// For a description of the `on_saved`, `validator`, or `autovalidate_mode` parameters, see
    /// `FormField`. For the rest, see [`DropdownMenu`].
    ///
    /// The `restoration_id` is used to restore the selected item; it is also passed to the
    /// inner text field.
    pub fn new(
        dropdown_menu_entries: impl Into<Rc<[DropdownMenuEntry<T>]>>,
    ) -> DropdownMenuFormField<T> {
        let dropdown_menu_entries = dropdown_menu_entries.into();
        let mut form_field = FormFieldData::new(Rc::new(DropdownMenuFormField::<T>::build_field));
        form_field.autovalidate_mode = AutovalidateMode::Disabled;
        DropdownMenuFormField {
            form_field,
            on_selected: None,
            controller: None,
            dropdown_menu: DropdownMenu::new(Rc::clone(&dropdown_menu_entries)),
            dropdown_menu_entries,
            label: None,
            hint_text: None,
            helper_text: None,
            decoration_builder: None,
        }
    }

    /// Dart `DropdownMenuFormField(key:)`.
    pub fn key(mut self, key: KeyRef) -> DropdownMenuFormField<T> {
        self.form_field.key = Some(key);
        self
    }

    /// Dart `DropdownMenuFormField(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.enabled(enabled);
        self
    }

    /// Dart `DropdownMenuFormField(width:)`.
    pub fn width(mut self, width: f64) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.width(width);
        self
    }

    /// Dart `DropdownMenuFormField(menuHeight:)`.
    pub fn menu_height(mut self, menu_height: f64) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.menu_height(menu_height);
        self
    }

    /// Dart `DropdownMenuFormField(leadingIcon:)`.
    pub fn leading_icon<K>(mut self, leading_icon: impl IntoWidget<K>) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.leading_icon(leading_icon);
        self
    }

    /// Dart `DropdownMenuFormField(trailingIcon:)`.
    pub fn trailing_icon<K>(
        mut self,
        trailing_icon: impl IntoWidget<K>,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.trailing_icon(trailing_icon);
        self
    }

    /// Dart `DropdownMenuFormField(showTrailingIcon:)`.
    pub fn show_trailing_icon(mut self, show_trailing_icon: bool) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.show_trailing_icon(show_trailing_icon);
        self
    }

    /// Dart `DropdownMenuFormField(trailingIconFocusNode:)`.
    pub fn trailing_icon_focus_node(
        mut self,
        trailing_icon_focus_node: Handle<FocusNode>,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self
            .dropdown_menu
            .trailing_icon_focus_node(trailing_icon_focus_node);
        self
    }

    /// Dart `DropdownMenuFormField(label:)`.
    pub fn label<K>(mut self, label: impl IntoWidget<K>) -> DropdownMenuFormField<T> {
        self.label = Some(label.into_widget());
        self
    }

    /// Dart `DropdownMenuFormField(hintText:)`.
    pub fn hint_text(mut self, hint_text: impl Into<String>) -> DropdownMenuFormField<T> {
        self.hint_text = Some(hint_text.into());
        self
    }

    /// Dart `DropdownMenuFormField(helperText:)`.
    pub fn helper_text(mut self, helper_text: impl Into<String>) -> DropdownMenuFormField<T> {
        self.helper_text = Some(helper_text.into());
        self
    }

    /// Dart `DropdownMenuFormField(selectedTrailingIcon:)`.
    pub fn selected_trailing_icon<K>(
        mut self,
        selected_trailing_icon: impl IntoWidget<K>,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self
            .dropdown_menu
            .selected_trailing_icon(selected_trailing_icon);
        self
    }

    /// Dart `DropdownMenuFormField(enableFilter:)`.
    pub fn enable_filter(mut self, enable_filter: bool) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.enable_filter(enable_filter);
        self
    }

    /// Dart `DropdownMenuFormField(enableSearch:)`.
    pub fn enable_search(mut self, enable_search: bool) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.enable_search(enable_search);
        self
    }

    /// Dart `DropdownMenuFormField(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.keyboard_type(keyboard_type);
        self
    }

    /// Dart `DropdownMenuFormField(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.scroll_padding(scroll_padding);
        self
    }

    /// Dart `DropdownMenuFormField(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.text_style(text_style);
        self
    }

    /// Dart `DropdownMenuFormField(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.text_align(text_align);
        self
    }

    /// Dart `DropdownMenuFormField(inputDecorationTheme:)`.
    pub fn input_decoration_theme(
        mut self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self
            .dropdown_menu
            .input_decoration_theme(input_decoration_theme);
        self
    }

    /// Dart `DropdownMenuFormField(decorationBuilder:)`.
    pub fn decoration_builder(
        mut self,
        decoration_builder: impl Fn(&mut App, BuildContext, AnyMenuController) -> InputDecoration
        + 'static,
    ) -> DropdownMenuFormField<T> {
        self.decoration_builder = Some(Rc::new(decoration_builder));
        self
    }

    /// Dart `DropdownMenuFormField(menuStyle:)`.
    pub fn menu_style(mut self, menu_style: MenuStyle) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.menu_style(menu_style);
        self
    }

    /// Dart `DropdownMenuFormField(controller:)`.
    pub fn controller(mut self, controller: AnyTextEditingController) -> DropdownMenuFormField<T> {
        self.controller = Some(controller);
        self
    }

    /// Dart `DropdownMenuFormField(initialSelection:)`.
    pub fn initial_selection(mut self, initial_selection: T) -> DropdownMenuFormField<T> {
        self.form_field.initial_value = Some(initial_selection);
        self
    }

    /// Dart `DropdownMenuFormField(onSelected:)`.
    pub fn on_selected(
        mut self,
        on_selected: impl Fn(&mut App, Option<T>) + 'static,
    ) -> DropdownMenuFormField<T> {
        self.on_selected = Some(Rc::new(on_selected));
        self
    }

    /// Dart `DropdownMenuFormField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: Handle<FocusNode>) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.focus_node(focus_node);
        self
    }

    /// Dart `DropdownMenuFormField(requestFocusOnTap:)`.
    pub fn request_focus_on_tap(mut self, request_focus_on_tap: bool) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self
            .dropdown_menu
            .request_focus_on_tap(request_focus_on_tap);
        self
    }

    /// Dart `DropdownMenuFormField(selectOnly:)`.
    pub fn select_only(mut self, select_only: bool) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.select_only(select_only);
        self
    }

    /// Dart `DropdownMenuFormField(expandedInsets:)`.
    pub fn expanded_insets(
        mut self,
        expanded_insets: impl Into<EdgeInsetsGeometry>,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.expanded_insets(expanded_insets);
        self
    }

    /// Dart `DropdownMenuFormField(alignmentOffset:)`.
    pub fn alignment_offset(mut self, alignment_offset: Offset) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.alignment_offset(alignment_offset);
        self
    }

    /// Dart `DropdownMenuFormField(filterCallback:)`.
    pub fn filter_callback(
        mut self,
        filter_callback: impl Fn(&mut App, &[DropdownMenuEntry<T>], &str) -> Vec<DropdownMenuEntry<T>>
        + 'static,
    ) -> DropdownMenuFormField<T> {
        let filter_callback: FilterCallback<T> = Rc::new(filter_callback);
        self.dropdown_menu.filter_callback = Some(filter_callback);
        self
    }

    /// Dart `DropdownMenuFormField(searchCallback:)`.
    pub fn search_callback(
        mut self,
        search_callback: impl Fn(&mut App, &[DropdownMenuEntry<T>], &str) -> Option<usize> + 'static,
    ) -> DropdownMenuFormField<T> {
        let search_callback: SearchCallback<T> = Rc::new(search_callback);
        self.dropdown_menu.search_callback = Some(search_callback);
        self
    }

    /// Dart `DropdownMenuFormField(inputFormatters:)`.
    pub fn input_formatters(
        mut self,
        input_formatters: Vec<TextInputFormatterRef>,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.input_formatters(input_formatters);
        self
    }

    /// Dart `DropdownMenuFormField(closeBehavior:)`.
    pub fn close_behavior(
        mut self,
        close_behavior: DropdownMenuCloseBehavior,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.close_behavior(close_behavior);
        self
    }

    /// Dart `DropdownMenuFormField(maxLines:)`.
    pub fn max_lines(mut self, max_lines: i32) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.max_lines(Some(max_lines));
        self
    }

    /// Dart `DropdownMenuFormField(textInputAction:)`.
    pub fn text_input_action(
        mut self,
        text_input_action: TextInputAction,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.text_input_action(text_input_action);
        self
    }

    /// Dart `DropdownMenuFormField(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.cursor_height(cursor_height);
        self
    }

    /// Dart `DropdownMenuFormField(menuController:)`.
    pub fn menu_controller(
        mut self,
        menu_controller: AnyMenuController,
    ) -> DropdownMenuFormField<T> {
        self.dropdown_menu = self.dropdown_menu.menu_controller(menu_controller);
        self
    }

    /// Dart `DropdownMenuFormField(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> DropdownMenuFormField<T> {
        self.form_field.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `DropdownMenuFormField(onSaved:)`.
    pub fn on_saved(
        mut self,
        on_saved: impl Fn(&mut App, Option<T>) + 'static,
    ) -> DropdownMenuFormField<T> {
        self.form_field.on_saved = Some(Rc::new(on_saved));
        self
    }

    /// Dart `DropdownMenuFormField(autovalidateMode:)`.
    pub fn autovalidate_mode(
        mut self,
        autovalidate_mode: AutovalidateMode,
    ) -> DropdownMenuFormField<T> {
        self.form_field.autovalidate_mode = autovalidate_mode;
        self
    }

    /// Dart `DropdownMenuFormField(validator:)`.
    pub fn validator(
        mut self,
        validator: impl Fn(&mut App, Option<&T>) -> Option<String> + 'static,
    ) -> DropdownMenuFormField<T> {
        self.form_field.validator = Some(Rc::new(validator));
        self
    }

    /// Dart `DropdownMenuFormField(forceErrorText:)`.
    pub fn force_error_text(
        mut self,
        force_error_text: impl Into<String>,
    ) -> DropdownMenuFormField<T> {
        self.form_field.force_error_text = Some(force_error_text.into());
        self
    }

    /// Dart `DropdownMenuFormField(errorBuilder:)`.
    pub fn error_builder(
        mut self,
        error_builder: impl Fn(&mut App, BuildContext, String) -> WidgetRef + 'static,
    ) -> DropdownMenuFormField<T> {
        self.form_field.error_builder = Some(Rc::new(error_builder));
        self
    }

    /// Dart's builder closure: the [`DropdownMenu`] for the form field's current state.
    fn build_field(app: &mut App, field: AnyFormFieldState<T>) -> WidgetRef {
        let state = field
            .downcast::<DropdownMenuFormFieldState<T>>(app)
            .expect("a DropdownMenuFormField's state is a DropdownMenuFormFieldState");
        let widget = state.widget(app);
        let decoration_builder = widget.decoration_builder.clone();
        let label = widget.label.clone();
        let hint_text = widget.hint_text.clone();
        let helper_text = widget.helper_text.clone();
        let error_builder = widget.form_field.error_builder.clone();
        let effective_decoration_builder =
            move |app: &mut App, context: BuildContext, menu_controller: AnyMenuController| {
                let decoration = match &decoration_builder {
                    Some(decoration_builder) => decoration_builder(app, context, menu_controller),
                    None => InputDecoration::new(),
                };
                let mut decoration_with_labels = decoration.copy_with();
                if let Some(label) = &label {
                    decoration_with_labels.label = Some(label.clone());
                }
                if let Some(hint_text) = &hint_text {
                    decoration_with_labels.hint_text = Some(hint_text.clone());
                }
                if let Some(helper_text) = &helper_text {
                    decoration_with_labels.helper_text = Some(helper_text.clone());
                }

                let Some(error_text) = state.error_text(app) else {
                    return decoration_with_labels;
                };

                match &error_builder {
                    Some(error_builder) => {
                        let error = error_builder(app, state.context(app), error_text);
                        decoration_with_labels.copy_with().error(error)
                    }
                    None => decoration_with_labels.copy_with().error_text(error_text),
                }
            };

        let widget = state.widget(app);
        let mut dropdown_menu = widget
            .dropdown_menu
            .clone()
            .decoration_builder(effective_decoration_builder)
            .controller(state.text_field_controller(app))
            .on_selected(move |app, value| state.did_change(app, value));
        dropdown_menu.restoration_id = state.widget(app).form_field.restoration_id.clone();
        dropdown_menu.initial_selection = state.value(app).cloned();

        UnmanagedRestorationScope::new(dropdown_menu)
            .bucket(state.bucket(app))
            .into_widget()
    }
}

impl<T: Clone + PartialEq + 'static> Debug for DropdownMenuFormField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuFormField")
            .field("form_field", &self.form_field)
            .field("controller", &self.controller)
            .field("dropdown_menu", &self.dropdown_menu)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> FormFieldLeaf for DropdownMenuFormField<T> {
    type Value = T;
    waft_widgets::form_field_accessors!();
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for DropdownMenuFormField<T> {
    type State = DropdownMenuFormFieldState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.form_field.key.as_ref()
    }

    fn create_state(&self) -> DropdownMenuFormFieldState<T> {
        DropdownMenuFormFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            form_field_state: FormFieldStateData::new(&self.form_field),
            restorable_controller: None,
            local_text_field_controller: None,
        }
    }
}

/// Dart's `_DropdownMenuFormFieldState`.
pub struct DropdownMenuFormFieldState<T: Clone + PartialEq + 'static> {
    state: StateData<DropdownMenuFormField<T>>,
    restoration: RestorationMixinData,
    form_field_state: FormFieldStateData<T>,
    // The controller used to restore the selected item.
    restorable_controller: Option<Handle<RestorableTextEditingController>>,
    // The controller used to reset the content of the DropdownMenu inner TextField.
    local_text_field_controller: Option<Handle<TextEditingController>>,
}

impl<T: Clone + PartialEq + 'static> DropdownMenuFormFieldState<T> {
    fn text_field_controller(self: Handle<Self>, app: &mut App) -> AnyTextEditingController {
        if let Some(controller) = self.widget(app).controller {
            return controller;
        }
        if let Some(controller) = app.get(self).local_text_field_controller {
            return controller.as_controller();
        }
        let controller = TextEditingController::new(app);
        app.get_mut(self).local_text_field_controller = Some(controller);
        controller.as_controller()
    }

    fn create_restorable_controller(self: Handle<Self>, app: &mut App, initial_value: Option<T>) {
        debug_assert!(app.get(self).restorable_controller.is_none());
        let label = self.find_label_by_value(app, initial_value.as_ref());
        let controller =
            RestorableTextEditingController::from_value(app, TextEditingValue::new().text(label));
        app.get_mut(self).restorable_controller = Some(controller);
        if !self.restore_pending(app) {
            self.register_restorable_controller(app);
        }
    }

    fn update_restorable_controller(self: Handle<Self>, app: &mut App, value: Option<T>) {
        if let Some(restorable_controller) = app.get(self).restorable_controller {
            let label = self.find_label_by_value(app, value.as_ref());
            RestorableListenable::value(restorable_controller, app)
                .as_controller()
                .set_value(app, TextEditingValue::new().text(label));
        }
    }

    fn register_restorable_controller(self: Handle<Self>, app: &mut App) {
        let controller = app
            .get(self)
            .restorable_controller
            .expect("register_restorable_controller needs the restorable controller");
        self.register_for_restoration(app, controller.as_property(), "controller");
    }

    fn find_value_by_label(self: Handle<Self>, app: &App, label: &str) -> Option<T> {
        self.widget(app)
            .dropdown_menu_entries
            .iter()
            .find(|entry| entry.label == label)
            .map(|entry| entry.value.clone())
    }

    fn find_label_by_value(self: Handle<Self>, app: &App, value: Option<&T>) -> String {
        self.widget(app)
            .dropdown_menu_entries
            .iter()
            .find(|entry| Some(&entry.value) == value)
            .map(|entry| entry.label.clone())
            .unwrap_or_default()
    }
}

impl<T: Clone + PartialEq + 'static> RestorationMixin for DropdownMenuFormFieldState<T> {
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
        if let Some(restorable_controller) = app.get(self).restorable_controller {
            self.register_restorable_controller(app);
            // Make sure to update the internal [DropdownMenuFieldState] value to sync up with
            // text editing controller value if it matches one of the item label.
            let text = RestorableListenable::value(restorable_controller, app)
                .as_controller()
                .text_value(app)
                .to_owned();
            if let Some(matching_value) = self.find_value_by_label(app, &text) {
                self.set_value(app, Some(matching_value));
            }
        }
    }
}

impl<T: Clone + PartialEq + 'static> FormFieldStateLeaf for DropdownMenuFormFieldState<T> {
    waft_widgets::form_field_state_accessors!();

    fn did_change(self: Handle<Self>, app: &mut App, value: Option<T>) {
        FormFieldState::did_change(self, app, value.clone());
        if let Some(on_selected) = self.widget(app).on_selected.clone() {
            on_selected(app, value.clone());
        }
        self.update_restorable_controller(app, value);
    }

    fn reset(self: Handle<Self>, app: &mut App) {
        FormFieldState::reset(self, app);
        if let Some(on_selected) = self.widget(app).on_selected.clone() {
            let value = self.value(app).cloned();
            on_selected(app, value);
        }
        let initial_value = self.widget(app).form_field.initial_value.clone();
        self.update_restorable_controller(app, initial_value.clone());
        if initial_value.is_none() {
            self.text_field_controller(app).clear(app);
        }
    }
}

impl<T: Clone + PartialEq + 'static> State for DropdownMenuFormFieldState<T> {
    type Widget = DropdownMenuFormField<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        FormFieldState::init_state(self, app);
        let initial_value = self.widget(app).form_field.initial_value.clone();
        self.create_restorable_controller(app, initial_value);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &DropdownMenuFormField<T>) {
        FormFieldState::did_update_widget(self, app, old_widget);
        let initial_value = self.widget(app).form_field.initial_value.clone();
        if old_widget.form_field.initial_value != initial_value && !self.has_interacted_by_user(app)
        {
            self.set_value(app, initial_value);
        }
        if old_widget.controller != self.widget(app).controller
            && let Some(controller) = app.get_mut(self).local_text_field_controller.take()
        {
            controller.as_controller().dispose(app);
            // The text field of the last build still listens to it until it updates.
            app.destroy_lingering(controller);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        FormFieldState::did_change_dependencies(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        FormFieldState::deactivate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(restorable_controller) = app.get(self).restorable_controller {
            let controller = RestorableListenable::value(restorable_controller, app);
            RestorableProperty::dispose(restorable_controller, app);
            app.destroy_lingering(controller);
            app.destroy_lingering(restorable_controller);
        }
        if let Some(controller) = app.get_mut(self).local_text_field_controller.take() {
            controller.as_controller().dispose(app);
            app.destroy_lingering(controller);
        }
        FormFieldState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        FormFieldState::build(self, app, context)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/dropdown_menu_form_field_test.dart`.

    use std::cell::Cell;
    use std::fmt::Debug;
    use std::time::Duration;

    use waft_embedder::{Rect, Size, TextAffinity, TextSelection};
    use waft_widgets::{AnyElement, Form, FormState, GlobalKey, Icon, SizedBox, Text};

    use super::*;
    use crate::icons::Icons;
    use crate::menu_anchor::MenuItemButton;
    use crate::scaffold::Scaffold;
    use crate::test_support::{
        WidgetTester, material_app, restorable_material_app, with_main_thread_stack,
    };
    use crate::text_field::TextField;
    use crate::tooltip::tests::{m3, tester};

    // ---- Dart's `MenuItem` and `menuEntries` ----

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum MenuItem {
        MenuItem0,
        MenuItem1,
        MenuItem2,
        MenuItem3,
    }

    impl MenuItem {
        const VALUES: [MenuItem; 4] = [
            MenuItem::MenuItem0,
            MenuItem::MenuItem1,
            MenuItem::MenuItem2,
            MenuItem::MenuItem3,
        ];

        fn label(self) -> &'static str {
            match self {
                MenuItem::MenuItem0 => "Item 0",
                MenuItem::MenuItem1 => "Item 1",
                MenuItem::MenuItem2 => "Item 2",
                MenuItem::MenuItem3 => "Item 3",
            }
        }
    }

    thread_local! {
        static MENU_ENTRIES: Rc<[DropdownMenuEntry<MenuItem>]> = MenuItem::VALUES
            .iter()
            .map(|value| DropdownMenuEntry::new(*value, value.label()))
            .collect();
    }

    /// Dart's one `menuEntries` list.
    fn menu_entries() -> Rc<[DropdownMenuEntry<MenuItem>]> {
        MENU_ENTRIES.with(Rc::clone)
    }

    fn field() -> DropdownMenuFormField<MenuItem> {
        DropdownMenuFormField::new(menu_entries())
    }

    // ---- finders and gestures ----

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    fn rect_of(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.get_rect(tester.render_box_of(element))
    }

    fn tap(tester: &mut WidgetTester, element: AnyElement) {
        let center = rect_of(tester, element).center();
        tester.tap(center, 1);
    }

    fn pump(tester: &mut WidgetTester) {
        tester.pump(Duration::ZERO);
    }

    /// Dart's `findMenuItemButton(label)`: the last `MenuItemButton` showing `label`.
    fn find_menu_item_button(tester: &WidgetTester, label: &str) -> AnyElement {
        let text = *tester.find_text(label).last().expect("the label");
        tester.ancestor_of_type::<MenuItemButton>(text)
    }

    /// Dart's `tester.tap(find.byType(DropdownMenu<T>))`.
    fn tap_dropdown_menu<T: 'static>(tester: &mut WidgetTester) {
        let menu = only(tester.find_by_type::<DropdownMenu<T>>());
        tap(tester, menu);
    }

    /// Dart's `tester.tap(findMenuItem(item))`.
    fn tap_item(tester: &mut WidgetTester, item: MenuItem) {
        let button = find_menu_item_button(tester, item.label());
        tap(tester, button);
    }

    fn pump_field<T: Clone + PartialEq + 'static>(
        tester: &mut WidgetTester,
        field: DropdownMenuFormField<T>,
    ) {
        let theme = m3(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Scaffold::new().body(field).into_widget(),
        ));
    }

    fn menu<R>(tester: &WidgetTester, read: impl FnOnce(&DropdownMenu<MenuItem>) -> R) -> R {
        let menu = only(tester.find_by_type::<DropdownMenu<MenuItem>>());
        tester.widget(menu, read)
    }

    fn text_field<R>(tester: &WidgetTester, read: impl FnOnce(&TextField) -> R) -> R {
        let field = only(tester.find_by_type::<TextField>());
        tester.widget(field, read)
    }

    fn field_state(
        tester: &WidgetTester,
        key: &GlobalKey,
    ) -> Handle<DropdownMenuFormFieldState<MenuItem>> {
        key.current_state::<DropdownMenuFormFieldState<MenuItem>>(&tester.cell.borrow())
            .expect("the field's state")
    }

    fn field_value(tester: &WidgetTester, key: &GlobalKey) -> Option<MenuItem> {
        let state = field_state(tester, key);
        state.value(&tester.cell.borrow()).copied()
    }

    /// The common shape of the 'Passes … to underlying DropdownMenu' cases: the menu of a
    /// default field reads `default`, the menu of the configured one `given`.
    fn assert_passes<R: PartialEq + Debug>(
        configure: fn(DropdownMenuFormField<MenuItem>) -> DropdownMenuFormField<MenuItem>,
        read: fn(&DropdownMenu<MenuItem>) -> R,
        default: fn() -> R,
        given: fn() -> R,
    ) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            pump_field(&mut tester, field());
            // Check default value.
            assert_eq!(menu(&tester, read), default());

            pump_field(&mut tester, configure(field()));
            assert_eq!(menu(&tester, read), given());
        });
    }

    /// Dart's `==` on a widget: identity.
    fn same(widget: &Option<WidgetRef>, expected: &WidgetRef) -> bool {
        widget
            .as_ref()
            .is_some_and(|widget| Rc::ptr_eq(widget, expected))
    }

    // ---- cases ----

    // 'Creates an underlying DropdownMenu'
    #[test]
    fn the_field_creates_a_dropdown_menu() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            assert_eq!(tester.find_by_type::<DropdownMenu<MenuItem>>().len(), 1);
        });
    }

    // 'Passes dropdownMenuEntries to underlying DropdownMenu'
    #[test]
    fn the_field_passes_its_entries() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            menu(&tester, |menu| {
                assert!(Rc::ptr_eq(&menu.dropdown_menu_entries, &menu_entries()));
            });
        });
    }

    // 'Dropdown menu can be opened and contains all the items'
    #[test]
    fn the_menu_opens_with_every_item() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            tap_dropdown_menu::<MenuItem>(&mut tester);
            pump(&mut tester);
            for item in MenuItem::VALUES {
                find_menu_item_button(&tester, item.label());
            }
        });
    }

    #[test]
    fn the_field_passes_enabled() {
        assert_passes(
            |field| field.enabled(false),
            |menu| menu.enabled,
            || true,
            || false,
        );
    }

    #[test]
    fn the_field_passes_width() {
        assert_passes(
            |field| field.width(100.0),
            |menu| menu.width,
            || None,
            || Some(100.0),
        );
    }

    #[test]
    fn the_field_passes_menu_height() {
        assert_passes(
            |field| field.menu_height(100.0),
            |menu| menu.menu_height,
            || None,
            || Some(100.0),
        );
    }

    /// 'Passes leadingIcon / trailingIcon / selectedTrailingIcon to underlying DropdownMenu':
    /// the menu holds the very widget.
    fn assert_passes_icon(
        configure: fn(
            DropdownMenuFormField<MenuItem>,
            WidgetRef,
        ) -> DropdownMenuFormField<MenuItem>,
        read: fn(&DropdownMenu<MenuItem>) -> Option<WidgetRef>,
    ) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            pump_field(&mut tester, field());
            // Check default value.
            assert!(menu(&tester, read).is_none());

            let icon = Icon::new(Some(Icons::abc())).into_widget();
            pump_field(&mut tester, configure(field(), Rc::clone(&icon)));
            assert!(same(&menu(&tester, read), &icon));
        });
    }

    #[test]
    fn the_field_passes_its_leading_icon() {
        assert_passes_icon(
            |field, icon| field.leading_icon(icon),
            |menu| menu.leading_icon.clone(),
        );
    }

    #[test]
    fn the_field_passes_its_trailing_icon() {
        assert_passes_icon(
            |field, icon| field.trailing_icon(icon),
            |menu| menu.trailing_icon.clone(),
        );
    }

    #[test]
    fn the_field_passes_its_selected_trailing_icon() {
        assert_passes_icon(
            |field, icon| field.selected_trailing_icon(icon),
            |menu| menu.selected_trailing_icon.clone(),
        );
    }

    #[test]
    fn the_field_passes_show_trailing_icon() {
        assert_passes(
            |field| field.show_trailing_icon(false),
            |menu| menu.show_trailing_icon,
            || true,
            || false,
        );
    }

    #[test]
    fn the_field_passes_the_trailing_icon_focus_node() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            assert_eq!(menu(&tester, |menu| menu.trailing_icon_focus_node), None);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_field(&mut tester, field().trailing_icon_focus_node(focus_node));
            assert_eq!(
                menu(&tester, |menu| menu.trailing_icon_focus_node),
                Some(focus_node)
            );
        });
    }

    // 'Passes label to underlying InputDecoration'
    #[test]
    fn the_field_passes_its_label_to_the_decoration() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            text_field(&tester, |text_field| {
                assert!(
                    text_field
                        .decoration
                        .as_ref()
                        .expect("a decoration")
                        .label
                        .is_none()
                );
            });

            let label = Text::new("Label").into_widget();
            pump_field(&mut tester, field().label(Rc::clone(&label)));
            text_field(&tester, |text_field| {
                assert!(same(
                    &text_field.decoration.as_ref().expect("a decoration").label,
                    &label
                ));
            });
        });
    }

    /// 'Passes hintText / helperText to underlying InputDecoration'.
    fn assert_passes_decoration_text(
        configure: fn(DropdownMenuFormField<MenuItem>, &str) -> DropdownMenuFormField<MenuItem>,
        read: fn(&InputDecoration) -> Option<String>,
    ) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            pump_field(&mut tester, field());
            text_field(&tester, |text_field| {
                assert_eq!(
                    read(text_field.decoration.as_ref().expect("a decoration")),
                    None
                );
            });
            pump_field(&mut tester, configure(field(), "Hint"));
            text_field(&tester, |text_field| {
                assert_eq!(
                    read(text_field.decoration.as_ref().expect("a decoration")).as_deref(),
                    Some("Hint")
                );
            });
        });
    }

    #[test]
    fn the_field_passes_its_hint_text_to_the_decoration() {
        assert_passes_decoration_text(
            |field, text| field.hint_text(text),
            |decoration| decoration.hint_text.clone(),
        );
    }

    #[test]
    fn the_field_passes_its_helper_text_to_the_decoration() {
        assert_passes_decoration_text(
            |field, text| field.helper_text(text),
            |decoration| decoration.helper_text.clone(),
        );
    }

    #[test]
    fn the_field_passes_enable_filter() {
        assert_passes(
            |field| field.enable_filter(true),
            |menu| menu.enable_filter,
            || false,
            || true,
        );
    }

    #[test]
    fn the_field_passes_enable_search() {
        assert_passes(
            |field| field.enable_search(false),
            |menu| menu.enable_search,
            || true,
            || false,
        );
    }

    #[test]
    fn the_field_passes_scroll_padding() {
        assert_passes(
            |field| field.scroll_padding(EdgeInsets::all(30.0)),
            |menu| menu.scroll_padding,
            || EdgeInsets::all(20.0),
            || EdgeInsets::all(30.0),
        );
    }

    #[test]
    fn the_field_passes_keyboard_type() {
        assert_passes(
            |field| field.keyboard_type(TextInputType::NUMBER),
            |menu| menu.keyboard_type,
            || None,
            || Some(TextInputType::NUMBER),
        );
    }

    // 'Passes filterCallback to underlying DropdownMenu'; Dart's `==` on a function is its
    // identity, which `Rc::ptr_eq` compares here.
    #[test]
    fn the_field_passes_its_filter_callback() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field().enable_filter(true));
            // Check default value.
            assert!(menu(&tester, |menu| menu.filter_callback.is_none()));

            let configured = field()
                .enable_filter(true)
                .filter_callback(|_app, entries, _filter| entries.to_vec());
            let filter_callback = configured
                .dropdown_menu
                .filter_callback
                .clone()
                .expect("set above");
            pump_field(&mut tester, configured);
            assert!(menu(&tester, |menu| menu
                .filter_callback
                .as_ref()
                .is_some_and(|callback| Rc::ptr_eq(callback, &filter_callback))));
        });
    }

    // 'Passes searchCallback to underlying DropdownMenu'
    #[test]
    fn the_field_passes_its_search_callback() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            // Check default value.
            assert!(menu(&tester, |menu| menu.search_callback.is_none()));

            let configured = field().search_callback(|_app, _entries, _filter| Some(0));
            let search_callback = configured
                .dropdown_menu
                .search_callback
                .clone()
                .expect("set above");
            pump_field(&mut tester, configured);
            assert!(menu(&tester, |menu| menu
                .search_callback
                .as_ref()
                .is_some_and(|callback| Rc::ptr_eq(callback, &search_callback))));
        });
    }

    // 'Passes inputFormatters to underlying DropdownMenu'; Dart's list is compared by
    // identity, here by each formatter's.
    #[test]
    fn the_field_passes_its_input_formatters() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            // Check default value.
            assert!(menu(&tester, |menu| menu.input_formatters.is_none()));

            let input_formatters: Vec<TextInputFormatterRef> = Vec::new();
            pump_field(
                &mut tester,
                field().input_formatters(input_formatters.clone()),
            );
            assert!(menu(&tester, |menu| menu
                .input_formatters
                .as_ref()
                .is_some_and(|formatters| formatters.len()
                    == input_formatters.len()
                    && formatters.iter().zip(&input_formatters).all(
                        |(given, expected)| Rc::ptr_eq(&given.0, &expected.0)
                    ))));
        });
    }

    fn bold() -> TextStyle {
        TextStyle::new().font_weight(waft_painting::FontWeight::BOLD)
    }

    #[test]
    fn the_field_passes_text_style() {
        assert_passes(
            |field| field.text_style(bold()),
            |menu| menu.text_style.clone(),
            || None,
            || Some(bold()),
        );
    }

    #[test]
    fn the_field_passes_text_align() {
        assert_passes(
            |field| field.text_align(TextAlign::Center),
            |menu| menu.text_align,
            || TextAlign::Start,
            || TextAlign::Center,
        );
    }

    #[test]
    fn the_field_passes_its_input_decoration_theme() {
        assert_passes(
            |field| field.input_decoration_theme(InputDecorationThemeData::new()),
            |menu| menu.input_decoration_theme.clone(),
            || None,
            || Some(InputDecorationThemeData::new()),
        );
    }

    // 'Calls the decoration builder to create the InputDecoration'
    #[test]
    fn the_field_decorates_with_its_decoration_builder() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());

            let decoration_builder_called = Rc::new(Cell::new(false));
            let called = Rc::clone(&decoration_builder_called);
            pump_field(
                &mut tester,
                field().decoration_builder(move |_app, _context, _controller| {
                    called.set(true);
                    InputDecoration::new().label_text("labelText")
                }),
            );

            // The decoration builder should have been called.
            assert!(decoration_builder_called.get());

            // The decoration label is the one provided by the decoration builder.
            text_field(&tester, |text_field| {
                assert_eq!(
                    text_field
                        .decoration
                        .as_ref()
                        .expect("a decoration")
                        .label_text
                        .as_deref(),
                    Some("labelText")
                );
            });
        });
    }

    #[test]
    fn the_field_passes_its_menu_style() {
        assert_passes(
            |field| field.menu_style(MenuStyle::new()),
            |menu| menu.menu_style.clone(),
            || None,
            || Some(MenuStyle::new()),
        );
    }

    // 'Passes controller to underlying DropdownMenu'
    #[test]
    fn the_field_passes_its_controller() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            let controller =
                TextEditingController::new(&mut tester.cell.borrow_mut()).as_controller();

            // Check default value.
            let default = menu(&tester, |menu| menu.controller);
            // A default controller is created.
            assert!(default.is_some());
            assert_ne!(default, Some(controller));

            pump_field(&mut tester, field().controller(controller));
            assert_eq!(menu(&tester, |menu| menu.controller), Some(controller));
        });
    }

    #[test]
    fn the_field_passes_its_focus_node() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            assert_eq!(menu(&tester, |menu| menu.focus_node), None);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_field(&mut tester, field().focus_node(focus_node));
            assert_eq!(menu(&tester, |menu| menu.focus_node), Some(focus_node));
        });
    }

    #[test]
    fn the_field_passes_request_focus_on_tap() {
        assert_passes(
            |field| field.request_focus_on_tap(true),
            |menu| menu.request_focus_on_tap,
            || None,
            || Some(true),
        );
    }

    #[test]
    fn the_field_passes_select_only() {
        assert_passes(
            |field| field.select_only(true),
            |menu| menu.select_only,
            || false,
            || true,
        );
    }

    #[test]
    fn the_field_passes_expanded_insets() {
        assert_passes(
            |field| field.expanded_insets(EdgeInsets::all(30.0)),
            |menu| menu.expanded_insets,
            || None,
            || Some(EdgeInsetsGeometry::all(30.0)),
        );
    }

    #[test]
    fn the_field_passes_its_alignment_offset() {
        assert_passes(
            |field| field.alignment_offset(Offset::new(10.0, 20.0)),
            |menu| menu.alignment_offset,
            || None,
            || Some(Offset::new(10.0, 20.0)),
        );
    }

    #[test]
    fn the_field_passes_its_close_behavior() {
        assert_passes(
            |field| field.close_behavior(DropdownMenuCloseBehavior::SelfOnly),
            |menu| menu.close_behavior,
            || DropdownMenuCloseBehavior::All,
            || DropdownMenuCloseBehavior::SelfOnly,
        );
    }

    #[test]
    fn the_field_passes_max_lines() {
        assert_passes(
            |field| field.max_lines(3),
            |menu| menu.max_lines,
            || Some(1),
            || Some(3),
        );
    }

    #[test]
    fn the_field_passes_its_text_input_action() {
        assert_passes(
            |field| field.text_input_action(TextInputAction::Next),
            |menu| menu.text_input_action,
            || None,
            || Some(TextInputAction::Next),
        );
    }

    #[test]
    fn the_field_passes_its_cursor_height() {
        assert_passes(
            |field| field.cursor_height(4.0),
            |menu| menu.cursor_height,
            || None,
            || Some(4.0),
        );
    }

    #[test]
    fn the_field_passes_its_menu_controller() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            assert_eq!(menu(&tester, |menu| menu.menu_controller), None);
            let menu_controller = waft_widgets::MenuControllerLeaf::as_menu_controller(
                waft_widgets::MenuController::new(&mut tester.cell.borrow_mut()),
            );
            pump_field(&mut tester, field().menu_controller(menu_controller));
            assert_eq!(
                menu(&tester, |menu| menu.menu_controller),
                Some(menu_controller)
            );
        });
    }

    #[test]
    fn the_field_passes_its_restoration_id() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_field(&mut tester, field());
            assert_eq!(menu(&tester, |menu| menu.restoration_id.clone()), None);

            pump_field(&mut tester, field().restoration_id("dropdown_menu"));
            assert_eq!(tester.find_by_type::<TextField>().len(), 1);
            assert_eq!(
                menu(&tester, |menu| menu.restoration_id.clone()).as_deref(),
                Some("dropdown_menu")
            );
        });
    }

    /// Opens the menu and taps `item`.
    fn select(tester: &mut WidgetTester, item: MenuItem) {
        tap_dropdown_menu::<MenuItem>(tester);
        pump(tester);
        tap_item(tester, item);
        pump(tester);
    }

    // 'Field state is correctly updated'
    #[test]
    fn a_selection_updates_the_field_value() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .initial_selection(MenuItem::MenuItem0),
            );

            select(&mut tester, MenuItem::MenuItem1);

            assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem1));
        });
    }

    fn saving_field(saved_value: &Rc<Cell<Option<MenuItem>>>) -> DropdownMenuFormField<MenuItem> {
        let saved = Rc::clone(saved_value);
        field()
            .initial_selection(saved_value.get().expect("an initial value"))
            .on_saved(move |_app, new_value| saved.set(new_value))
    }

    // 'onSaved callback is called when the field is outside a Form'
    #[test]
    fn saving_a_field_outside_a_form_calls_on_saved() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            let saved_value = Rc::new(Cell::new(Some(MenuItem::MenuItem0)));
            pump_field(
                &mut tester,
                saving_field(&saved_value).key(field_key.clone().into()),
            );

            select(&mut tester, MenuItem::MenuItem1);
            assert_eq!(saved_value.get(), Some(MenuItem::MenuItem0));

            let state = field_state(&tester, &field_key);
            state.save(&mut tester.cell.borrow_mut());
            pump(&mut tester);

            assert_eq!(saved_value.get(), Some(MenuItem::MenuItem1));
        });
    }

    // 'onSaved callback is called when the field is inside a Form'
    #[test]
    fn saving_the_form_calls_on_saved() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let form_key = GlobalKey::new();
            let saved_value = Rc::new(Cell::new(Some(MenuItem::MenuItem0)));
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(Form::new(saving_field(&saved_value)).key(form_key.clone().into()))
                    .into_widget(),
            ));

            select(&mut tester, MenuItem::MenuItem1);
            assert_eq!(saved_value.get(), Some(MenuItem::MenuItem0));

            let form = form_key
                .current_state::<FormState>(&tester.cell.borrow())
                .expect("the form's state");
            form.save(&mut tester.cell.borrow_mut());
            pump(&mut tester);

            assert_eq!(saved_value.get(), Some(MenuItem::MenuItem1));
        });
    }

    // 'Field can be reset'
    #[test]
    fn a_reset_restores_the_initial_selection() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            let saved_value = Rc::new(Cell::new(Some(MenuItem::MenuItem0)));
            pump_field(
                &mut tester,
                saving_field(&saved_value).key(field_key.clone().into()),
            );

            select(&mut tester, MenuItem::MenuItem1);
            assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem1));

            let state = field_state(&tester, &field_key);
            state.reset(&mut tester.cell.borrow_mut());
            pump(&mut tester);

            assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem0));
        });
    }

    fn collapsed_value(text: &str) -> TextEditingValue {
        TextEditingValue::new()
            .text(text)
            .selection(TextSelection::collapsed(
                text.encode_utf16().count() as i32,
                TextAffinity::Downstream,
            ))
    }

    /// 'Inner text field is cleared on reset when initialSelection is null' with the default
    /// controller (`None`) or a custom one.
    fn assert_a_reset_clears_the_text_field(custom_controller: bool) {
        let mut tester = tester();
        let field_key = GlobalKey::new();
        let controller = custom_controller
            .then(|| TextEditingController::new(&mut tester.cell.borrow_mut()).as_controller());
        let mut form_field = field().key(field_key.clone().into());
        if let Some(controller) = controller {
            form_field = form_field.controller(controller);
        }
        pump_field(&mut tester, form_field);

        let text_field_controller = text_field(&tester, |text_field| text_field.controller)
            .expect("the field's controller");
        if let Some(controller) = controller {
            // Custom controller is correctly passed to the inner TextField.
            assert_eq!(text_field_controller, controller);
        }

        // Select menuItem1.
        select(&mut tester, MenuItem::MenuItem1);
        assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem1));
        assert_eq!(
            text_field_controller.value(&tester.cell.borrow()),
            &collapsed_value("Item 1")
        );

        // After reset the text field content is cleared.
        let state = field_state(&tester, &field_key);
        state.reset(&mut tester.cell.borrow_mut());
        pump(&mut tester);

        assert_eq!(field_value(&tester, &field_key), None);
        assert_eq!(
            text_field_controller.value(&tester.cell.borrow()),
            &collapsed_value("")
        );
    }

    // 'Inner text field is cleared on reset when initialSelection is null - Default controller'
    #[test]
    fn a_reset_without_an_initial_selection_clears_the_default_controller() {
        with_main_thread_stack(|| assert_a_reset_clears_the_text_field(false));
    }

    // 'Inner text field is cleared on reset when initialSelection is null - Custom controller'
    #[test]
    fn a_reset_without_an_initial_selection_clears_a_given_controller() {
        with_main_thread_stack(|| assert_a_reset_clears_the_text_field(true));
    }

    // 'isValid and hasError results are correct'
    #[test]
    fn is_valid_and_has_error_follow_the_validator() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .autovalidate_mode(AutovalidateMode::Always),
            );

            // No validation error.
            let state = field_state(&tester, &field_key);
            assert!(state.is_valid(&mut tester.cell.borrow_mut()));
            assert!(!state.has_error(&tester.cell.borrow()));

            let validation_error = "Required";
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .autovalidate_mode(AutovalidateMode::Always)
                    .validator(move |_app, _item| Some(validation_error.to_owned())),
            );

            // Validation error.
            let state = field_state(&tester, &field_key);
            assert!(!state.is_valid(&mut tester.cell.borrow_mut()));
            assert!(state.has_error(&tester.cell.borrow()));
        });
    }

    // 'Validation result is shown as error text'
    #[test]
    fn the_validation_result_shows_as_the_error_text() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            let validation_error = "Required";
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .autovalidate_mode(AutovalidateMode::Always)
                    .validator(move |_app, _item| Some(validation_error.to_owned())),
            );

            let state = field_state(&tester, &field_key);
            state.validate(&mut tester.cell.borrow_mut());
            pump(&mut tester);

            assert_eq!(tester.find_text(validation_error).len(), 1);
            text_field(&tester, |text_field| {
                assert_eq!(
                    text_field
                        .decoration
                        .as_ref()
                        .expect("a decoration")
                        .error_text
                        .as_deref(),
                    Some(validation_error)
                );
            });
        });
    }

    // 'Validation result is shown as error widget created by errorBuilder'
    #[test]
    fn the_error_builder_builds_the_validation_error() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            let validation_error = "Required";
            let error_key: KeyRef = waft_foundation::UniqueKey::new().into();
            let error_builder_called = Rc::new(Cell::new(false));
            let called = Rc::clone(&error_builder_called);
            let key = error_key.clone();
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .validator(move |_app, _item| Some(validation_error.to_owned()))
                    .error_builder(move |_app, _context, error_text| {
                        called.set(true);
                        Text::new(error_text).key(key.clone()).into_widget()
                    }),
            );

            let error_found = |tester: &WidgetTester| {
                let found = tester.elements();
                let app = tester.cell.borrow();
                found
                    .into_iter()
                    .filter(|element| element.widget(&app).key() == Some(&error_key))
                    .count()
            };
            assert!(!error_builder_called.get());
            assert_eq!(error_found(&tester), 0);

            let state = field_state(&tester, &field_key);
            state.validate(&mut tester.cell.borrow_mut());
            pump(&mut tester);

            assert!(error_builder_called.get());
            assert_eq!(error_found(&tester), 1);
            text_field(&tester, |text_field| {
                let error = text_field
                    .decoration
                    .as_ref()
                    .expect("a decoration")
                    .error
                    .clone()
                    .expect("an error widget");
                assert_eq!(error.key(), Some(&error_key));
            });
        });
    }

    // 'Initial selection is applied'
    #[test]
    fn the_initial_selection_is_the_field_value() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .initial_selection(MenuItem::MenuItem0),
            );
            assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem0));
        });
    }

    // 'Initial selection is applied when updated and the field has not been updated
    // in-between'
    #[test]
    fn a_new_initial_selection_applies_before_the_user_selects() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            for item in [MenuItem::MenuItem0, MenuItem::MenuItem1] {
                pump_field(
                    &mut tester,
                    field()
                        .key(field_key.clone().into())
                        .initial_selection(item),
                );
                assert_eq!(field_value(&tester, &field_key), Some(item));
            }
        });
    }

    // 'Initial selection is not applied when updated and the field has been updated
    // in-between'
    #[test]
    fn a_new_initial_selection_does_not_replace_the_user_selection() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .initial_selection(MenuItem::MenuItem0),
            );
            assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem0));

            // Select a different item than the initial one.
            select(&mut tester, MenuItem::MenuItem2);
            assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem2));

            // Update initial selection.
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .initial_selection(MenuItem::MenuItem1),
            );

            // The value selected by the user is preserved.
            assert_eq!(field_value(&tester, &field_key), Some(MenuItem::MenuItem2));
        });
    }

    /// Dart's `tester.pumpWidget(MaterialApp(restorationScopeId: 'app', home: …))` of the
    /// restorable field on a tester whose host restores `data`: a new one for each
    /// `restartAndRestore` and `restoreFrom`.
    fn restored(
        data: Option<waft_embedder::RestorationMap>,
        field_key: &GlobalKey,
    ) -> WidgetTester {
        let mut tester = WidgetTester::restoring(data);
        tester.install_platform_fonts();
        let theme = m3(&tester.cell.borrow());
        tester.pump_widget(restorable_material_app(
            "app",
            theme,
            Scaffold::new()
                .body(
                    field()
                        .key(field_key.clone().into())
                        .initial_selection(MenuItem::MenuItem0)
                        .restoration_id("dropdown_menu_form_field"),
                )
                .into_widget(),
        ));
        tester
    }

    // 'Selected value is restorable'
    #[test]
    fn the_selected_value_is_restored() {
        with_main_thread_stack(|| {
            let form_field_state = GlobalKey::new();
            let mut tester = restored(None, &form_field_state);
            assert_eq!(
                field_value(&tester, &form_field_state),
                Some(MenuItem::MenuItem0)
            );

            // Select a different item than the initial one.
            select(&mut tester, MenuItem::MenuItem2);
            assert_eq!(
                field_value(&tester, &form_field_state),
                Some(MenuItem::MenuItem2)
            );

            // Needed for restoration data to be updated.
            pump(&mut tester);

            let data = tester.restoration_data();
            let tester = restored(data.clone(), &form_field_state);
            assert_eq!(
                field_value(&tester, &form_field_state),
                Some(MenuItem::MenuItem2)
            );

            let state = field_state(&tester, &form_field_state);
            state.reset(&mut tester.cell.borrow_mut());
            assert_eq!(
                field_value(&tester, &form_field_state),
                Some(MenuItem::MenuItem0)
            );

            let mut tester = restored(data, &form_field_state);
            pump(&mut tester);
            assert_eq!(
                field_value(&tester, &form_field_state),
                Some(MenuItem::MenuItem2)
            );
        });
    }

    // 'onSelect is called exactly once when a selection is made.'
    #[test]
    fn a_selection_calls_on_selected_once() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let on_selected_call_count = Rc::new(Cell::new(0));
            let count = Rc::clone(&on_selected_call_count);
            pump_field(
                &mut tester,
                field()
                    .initial_selection(MenuItem::MenuItem0)
                    .on_selected(move |_app, _value| count.set(count.get() + 1)),
            );
            // Select a different item than the initial one.
            select(&mut tester, MenuItem::MenuItem2);
            assert_eq!(on_selected_call_count.get(), 1);
        });
    }

    // 'onSelect is called exactly once when reset'
    #[test]
    fn a_reset_calls_on_selected_once() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            let on_selected_call_count = Rc::new(Cell::new(0));
            let count = Rc::clone(&on_selected_call_count);
            pump_field(
                &mut tester,
                field()
                    .key(field_key.clone().into())
                    .on_selected(move |_app, _value| count.set(count.get() + 1)),
            );

            let state = field_state(&tester, &field_key);
            state.reset(&mut tester.cell.borrow_mut());
            pump(&mut tester);
            assert_eq!(on_selected_call_count.get(), 1);
        });
    }

    // 'DropdownMenuFormField does not crash at zero area'
    #[test]
    fn a_field_at_zero_area_does_not_panic() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(waft_widgets::Center::new().child(SizedBox::shrink().child(field())))
                    .into_widget(),
            ));
            let form_field = only(tester.find_by_type::<DropdownMenuFormField<MenuItem>>());
            let size = tester.render_box_of(form_field).size(&tester.cell.borrow());
            assert_eq!(size, Size::ZERO);
        });
    }

    // 'Allow null entry to clear selection'
    #[test]
    fn an_entry_with_a_null_value_can_be_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller =
                TextEditingController::new(&mut tester.cell.borrow_mut()).as_controller();
            let select_none_label = "Select none";
            let nullable_menu_items: Rc<[DropdownMenuEntry<Option<String>>]> = Rc::from(vec![
                DropdownMenuEntry::new(None, select_none_label),
                DropdownMenuEntry::new(Some("a".to_owned()), "A"),
                DropdownMenuEntry::new(Some("b".to_owned()), "B"),
            ]);
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(waft_widgets::StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            DropdownMenuFormField::new(Rc::clone(&nullable_menu_items))
                                .controller(controller)
                                .request_focus_on_tap(true)
                                .enable_filter(true)
                                .on_selected(move |app, _value| {
                                    set_state(app, &mut |_app| {});
                                })
                                .into_widget()
                        },
                    ))
                    .into_widget(),
            ));

            // Open the menu.
            let form_field = only(tester.find_by_type::<DropdownMenuFormField<Option<String>>>());
            tap(&mut tester, form_field);
            pump(&mut tester);

            // Select the 'None' item.
            let button = find_menu_item_button(&tester, select_none_label);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert_eq!(
                controller.text_value(&tester.cell.borrow()),
                select_none_label
            );
        });
    }
}
