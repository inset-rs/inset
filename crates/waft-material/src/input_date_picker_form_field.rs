//! Flutter counterpart: material_ui's `input_date_picker_form_field.dart`.
//!
//! A text field that takes a date typed in the locale's compact format, validated against a
//! date range, for the input mode of the date pickers.

use std::fmt;
use std::rc::Rc;

use waft_embedder::{TextEditingValue, TextInputType, TextSelection};
use waft_foundation::{App, DateTime, Handle, Ref, ValueChanged};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_widgets::{
    BuildContext, FocusNodeRef, IntoWidget, KeyRef, SelectableDayPredicate, Semantics, State,
    StateData, StatefulWidget, TextEditingController, TextEditingControllerLeaf, WidgetRef,
};

use crate::date::{CalendarDelegate, GregorianCalendarDelegate, is_day_before};
use crate::input_border::{InputBorderRef, OutlineInputBorder, UnderlineInputBorder};
use crate::input_decorator::{InputDecoration, InputDecorationTheme};
use crate::material_localizations::MaterialLocalizations;
use crate::text_form_field::TextFormField;
use crate::theme::Theme;

/// A [`TextFormField`] configured to accept and validate a date entered by a user.
///
/// When the field is saved or submitted, the text will be parsed into a [`DateTime`] according
/// to the ambient locale's compact date format. If the input text doesn't parse into a date,
/// the [`error_format_text`](Self::error_format_text) message will be displayed under the field.
///
/// [`first_date`](Self::first_date), [`last_date`](Self::last_date), and
/// [`selectable_day_predicate`](Self::selectable_day_predicate) provide constraints on the
/// dates that are valid. If the input date isn't in the date range or doesn't pass the given
/// predicate, then the [`error_invalid_text`](Self::error_invalid_text) message will be
/// displayed under the field.
///
/// In addition to validating the user input text, this widget can accept user input
/// through the keyboard or other input methods, via the field's text editing; the date it
/// shows is the [`initial_date`](Self::initial_date), formatted.
///
/// Requires a [`MaterialLocalizations`] and a [`Directionality`](waft_widgets::Directionality)
/// ancestor.
///
/// See also:
///
///  * `showDatePicker`, which shows a dialog that contains a Material Design date picker which
///    includes support for text entry of dates.
pub struct InputDatePickerFormField {
    pub key: Option<KeyRef>,

    /// If provided, it will be used as the default value of the field.
    pub initial_date: Option<DateTime>,

    /// The earliest allowable [`DateTime`] that the user can input.
    pub first_date: DateTime,

    /// The latest allowable [`DateTime`] that the user can input.
    pub last_date: DateTime,

    /// An optional method to call when the user indicates they are done editing the text in
    /// the field. Will only be called if the input represents a valid [`DateTime`].
    pub on_date_submitted: Option<ValueChanged<DateTime>>,

    /// An optional method to call with the final date when the form is saved via
    /// `FormState::save`. Will only be called if the input represents a valid [`DateTime`].
    pub on_date_saved: Option<ValueChanged<DateTime>>,

    /// Function to provide full control over which [`DateTime`] can be selected.
    pub selectable_day_predicate: Option<SelectableDayPredicate>,

    /// The error text displayed if the entered date is not in the correct format.
    pub error_format_text: Option<String>,

    /// The error text displayed if the date is not valid.
    ///
    /// A date is not valid if it is earlier than [`first_date`](Self::first_date), later than
    /// [`last_date`](Self::last_date), or doesn't pass the
    /// [`selectable_day_predicate`](Self::selectable_day_predicate).
    pub error_invalid_text: Option<String>,

    /// The hint text displayed in the [`TextField`](crate::TextField).
    ///
    /// If this is null, it will default to the date format string. For example, 'mm/dd/yyyy'
    /// for en_US.
    pub field_hint_text: Option<String>,

    /// The label text displayed in the [`TextField`](crate::TextField).
    ///
    /// If this is null, it will default to the words representing the date format string.
    /// For example, 'Enter Date' for en_US.
    pub field_label_text: Option<String>,

    /// The keyboard type of the [`TextField`](crate::TextField).
    ///
    /// If this is null, it will default to [`TextInputType::DATETIME`].
    pub keyboard_type: Option<TextInputType>,

    /// Whether this field should focus itself if nothing else is already focused.
    ///
    /// If true, the keyboard will open as soon as this input obtains focus. Otherwise, the
    /// keyboard is only shown after the user taps the text field.
    ///
    /// Defaults to false.
    pub autofocus: bool,

    /// Determines if an empty date would show [`error_format_text`](Self::error_format_text) or
    /// not.
    ///
    /// Defaults to false.
    ///
    /// When [`autofocus`](Self::autofocus) is true and the date is empty, an error is not
    /// shown.
    pub accept_empty_date: bool,

    /// Focus node of the field.
    pub focus_node: Option<FocusNodeRef>,

    /// The calendar system to use.
    ///
    /// Defaults to [`GregorianCalendarDelegate`].
    pub calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl InputDatePickerFormField {
    /// Creates a [`TextFormField`] configured to accept and validate a date; Dart's optional
    /// named arguments are the setters.
    ///
    /// If the optional [`initial_date`](Self::initial_date) is provided, then it will be used
    /// to populate the text field. If the [`initial_date`](Self::initial_date) is outside of
    /// the allowed range, it will be shown as the initial text, and an error will be shown.
    pub fn new(first_date: DateTime, last_date: DateTime) -> InputDatePickerFormField {
        InputDatePickerFormField {
            key: None,
            initial_date: None,
            first_date,
            last_date,
            on_date_submitted: None,
            on_date_saved: None,
            selectable_day_predicate: None,
            error_format_text: None,
            error_invalid_text: None,
            field_hint_text: None,
            field_label_text: None,
            keyboard_type: None,
            autofocus: false,
            accept_empty_date: false,
            focus_node: None,
            calendar_delegate: Rc::new(GregorianCalendarDelegate),
        }
    }

    /// Dart's `initialDate` field, as its constructor fills it: the day of the date given.
    pub fn effective_initial_date(&self, app: &App) -> Option<DateTime> {
        self.initial_date
            .map(|date| self.calendar_delegate.date_only(app, &date))
    }

    /// Dart's `firstDate` field, as its constructor fills it: the day of the date given.
    pub fn effective_first_date(&self, app: &App) -> DateTime {
        self.calendar_delegate.date_only(app, &self.first_date)
    }

    /// Dart's `lastDate` field, as its constructor fills it: the day of the date given.
    pub fn effective_last_date(&self, app: &App) -> DateTime {
        self.calendar_delegate.date_only(app, &self.last_date)
    }

    /// Dart's constructor assert that the initial day satisfies `selectableDayPredicate`, which
    /// takes the day `date_only` makes, so it runs once the `App` is at hand.
    fn debug_check_initial_date_is_selectable(&self, app: &App) {
        debug_assert!(
            self.selectable_day_predicate
                .as_ref()
                .is_none_or(|predicate| self
                    .effective_initial_date(app)
                    .is_none_or(|date| predicate(app, date))),
            "Provided initialDate {:?} must satisfy provided selectableDayPredicate.",
            self.effective_initial_date(app)
        );
    }

    /// Dart `InputDatePickerFormField(key:)`.
    pub fn key(mut self, key: KeyRef) -> InputDatePickerFormField {
        self.key = Some(key);
        self
    }

    /// Dart `InputDatePickerFormField(initialDate:)`.
    pub fn initial_date(mut self, initial_date: DateTime) -> InputDatePickerFormField {
        self.initial_date = Some(initial_date);
        self
    }

    /// Dart `InputDatePickerFormField(onDateSubmitted:)`.
    pub fn on_date_submitted(
        mut self,
        on_date_submitted: ValueChanged<DateTime>,
    ) -> InputDatePickerFormField {
        self.on_date_submitted = Some(on_date_submitted);
        self
    }

    /// Dart `InputDatePickerFormField(onDateSaved:)`.
    pub fn on_date_saved(
        mut self,
        on_date_saved: ValueChanged<DateTime>,
    ) -> InputDatePickerFormField {
        self.on_date_saved = Some(on_date_saved);
        self
    }

    /// Dart `InputDatePickerFormField(selectableDayPredicate:)`.
    pub fn selectable_day_predicate(
        mut self,
        selectable_day_predicate: SelectableDayPredicate,
    ) -> InputDatePickerFormField {
        self.selectable_day_predicate = Some(selectable_day_predicate);
        self
    }

    /// Dart `InputDatePickerFormField(errorFormatText:)`.
    pub fn error_format_text(
        mut self,
        error_format_text: impl Into<String>,
    ) -> InputDatePickerFormField {
        self.error_format_text = Some(error_format_text.into());
        self
    }

    /// Dart `InputDatePickerFormField(errorInvalidText:)`.
    pub fn error_invalid_text(
        mut self,
        error_invalid_text: impl Into<String>,
    ) -> InputDatePickerFormField {
        self.error_invalid_text = Some(error_invalid_text.into());
        self
    }

    /// Dart `InputDatePickerFormField(fieldHintText:)`.
    pub fn field_hint_text(
        mut self,
        field_hint_text: impl Into<String>,
    ) -> InputDatePickerFormField {
        self.field_hint_text = Some(field_hint_text.into());
        self
    }

    /// Dart `InputDatePickerFormField(fieldLabelText:)`.
    pub fn field_label_text(
        mut self,
        field_label_text: impl Into<String>,
    ) -> InputDatePickerFormField {
        self.field_label_text = Some(field_label_text.into());
        self
    }

    /// Dart `InputDatePickerFormField(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> InputDatePickerFormField {
        self.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `InputDatePickerFormField(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> InputDatePickerFormField {
        self.autofocus = autofocus;
        self
    }

    /// Dart `InputDatePickerFormField(acceptEmptyDate:)`.
    pub fn accept_empty_date(mut self, accept_empty_date: bool) -> InputDatePickerFormField {
        self.accept_empty_date = accept_empty_date;
        self
    }

    /// Dart `InputDatePickerFormField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> InputDatePickerFormField {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `InputDatePickerFormField(calendarDelegate:)`.
    pub fn calendar_delegate(
        mut self,
        calendar_delegate: Rc<dyn CalendarDelegate>,
    ) -> InputDatePickerFormField {
        self.calendar_delegate = calendar_delegate;
        self
    }
}

impl fmt::Debug for InputDatePickerFormField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputDatePickerFormField")
            .field("initial_date", &self.initial_date)
            .field("first_date", &self.first_date)
            .field("last_date", &self.last_date)
            .field("autofocus", &self.autofocus)
            .field("accept_empty_date", &self.accept_empty_date)
            .finish_non_exhaustive()
    }
}

impl InputDatePickerFormField {
    /// Dart's constructor asserts on the dates, by their days, which are read in the local time
    /// zone: run in `init_state` and `did_update_widget`, for every widget the state is given,
    /// since a constructor has no [`App`].
    fn debug_check_dates(&self, app: &App) {
        debug_assert!(
            !is_day_before(app, &self.last_date, &self.first_date),
            "lastDate {} must be on or after firstDate {}.",
            self.last_date.to_string(app),
            self.first_date.to_string(app)
        );
        debug_assert!(
            self.initial_date
                .is_none_or(|date| !is_day_before(app, &date, &self.first_date)),
            "initialDate {:?} must be on or after firstDate {}.",
            self.initial_date.map(|date| date.to_string(app)),
            self.first_date.to_string(app)
        );
        debug_assert!(
            self.initial_date
                .is_none_or(|date| !is_day_before(app, &self.last_date, &date)),
            "initialDate {:?} must be on or before lastDate {}.",
            self.initial_date.map(|date| date.to_string(app)),
            self.last_date.to_string(app)
        );
    }
}

impl StatefulWidget for InputDatePickerFormField {
    type State = InputDatePickerFormFieldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> InputDatePickerFormFieldState {
        InputDatePickerFormFieldState {
            state: StateData::new(),
            controller: None,
            selected_date: None,
            input_text: None,
            auto_selected: false,
        }
    }
}

/// Dart's `_InputDatePickerFormFieldState`.
pub struct InputDatePickerFormFieldState {
    state: StateData<InputDatePickerFormField>,
    /// Created in `init_state`, because a state is created without the [`App`].
    controller: Option<Ref<TextEditingController>>,
    selected_date: Option<DateTime>,
    input_text: Option<String>,
    auto_selected: bool,
}

impl InputDatePickerFormFieldState {
    fn controller(self: Handle<Self>, app: &App) -> Ref<TextEditingController> {
        app.get(self)
            .controller
            .clone()
            .expect("created in init_state")
    }

    fn update_value_for_selected_date(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let controller = self.controller(app).handle();
        if let Some(selected_date) = app.get(self).selected_date {
            let localizations = <dyn MaterialLocalizations>::of(app, context);
            let widget = self.widget(app);
            let autofocus = widget.autofocus;
            let input_text =
                widget
                    .calendar_delegate
                    .format_compact_date(app, &selected_date, &*localizations);
            let mut text_editing_value = TextEditingValue::new().text(input_text.clone());
            // Select the new text if we are auto focused and haven't selected the text before.
            if autofocus && !app.get(self).auto_selected {
                text_editing_value = text_editing_value.selection(TextSelection::new(
                    0,
                    input_text.encode_utf16().count() as i32,
                ));
                app.get_mut(self).auto_selected = true;
            }
            app.get_mut(self).input_text = Some(input_text);
            controller.set_value(app, text_editing_value);
        } else {
            app.get_mut(self).input_text = Some(String::new());
            controller.set_value(app, TextEditingValue::new().text(""));
        }
    }

    fn parse_date(self: Handle<Self>, app: &mut App, text: Option<&str>) -> Option<DateTime> {
        let context = self.context(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        self.widget(app)
            .calendar_delegate
            .parse_compact_date(app, text, &*localizations)
    }

    fn is_valid_acceptable_date(self: Handle<Self>, app: &App, date: Option<&DateTime>) -> bool {
        let widget = self.widget(app);
        date.is_some_and(|date| {
            !date.is_before(&widget.effective_first_date(app))
                && !date.is_after(&widget.effective_last_date(app))
                && widget
                    .selectable_day_predicate
                    .as_ref()
                    .is_none_or(|predicate| predicate(app, *date))
        })
    }

    fn validate_date(self: Handle<Self>, app: &mut App, text: Option<&String>) -> Option<String> {
        if text.is_none_or(|text| text.is_empty()) && self.widget(app).accept_empty_date {
            return None;
        }
        let context = self.context(app);
        let date = self.parse_date(app, text.map(String::as_str));
        match date {
            None => Some(
                self.widget(app)
                    .error_format_text
                    .clone()
                    .unwrap_or_else(|| {
                        <dyn MaterialLocalizations>::of(app, context).invalid_date_format_label()
                    }),
            ),
            Some(date) if !self.is_valid_acceptable_date(app, Some(&date)) => Some(
                self.widget(app)
                    .error_invalid_text
                    .clone()
                    .unwrap_or_else(|| {
                        <dyn MaterialLocalizations>::of(app, context).date_out_of_range_label()
                    }),
            ),
            Some(_) => None,
        }
    }

    fn update_date(
        self: Handle<Self>,
        app: &mut App,
        text: Option<&str>,
        callback: Option<ValueChanged<DateTime>>,
    ) {
        let date = self.parse_date(app, text);
        if self.is_valid_acceptable_date(app, date.as_ref()) {
            app.get_mut(self).selected_date = date;
            app.get_mut(self).input_text = text.map(str::to_owned);
            if let (Some(callback), Some(date)) = (callback, date) {
                callback(app, date);
            }
        }
    }

    fn handle_saved(self: Handle<Self>, app: &mut App, text: Option<String>) {
        let callback = self.widget(app).on_date_saved.clone();
        self.update_date(app, text.as_deref(), callback);
    }

    fn handle_submitted(self: Handle<Self>, app: &mut App, text: String) {
        let callback = self.widget(app).on_date_submitted.clone();
        self.update_date(app, Some(&text), callback);
    }
}

impl State for InputDatePickerFormFieldState {
    type Widget = InputDatePickerFormField;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.widget(app).debug_check_dates(app);
        self.widget(app).debug_check_initial_date_is_selectable(app);
        let controller = TextEditingController::new(app);
        let initial_date = self.widget(app).effective_initial_date(app);
        let state = app.get_mut(self);
        state.controller = Some(controller);
        state.selected_date = initial_date;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.controller(app).handle().dispose(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.update_value_for_selected_date(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &InputDatePickerFormField) {
        self.widget(app).debug_check_dates(app);
        self.widget(app).debug_check_initial_date_is_selectable(app);
        if self.widget(app).effective_initial_date(app) != old_widget.effective_initial_date(app) {
            // Can't update the form field in the middle of a build, so do it next frame
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _duration| {
                    if !self.mounted(app) {
                        return;
                    }
                    let initial_date = self.widget(app).effective_initial_date(app);
                    app.get_mut(self).selected_date = initial_date;
                    self.update_value_for_selected_date(app);
                    self.set_state(app, |_| {});
                }),
                None,
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let use_material3 = theme.use_material3;
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let date_picker_theme = theme.date_picker_theme.clone();
        let input_theme = InputDecorationTheme::of(app, context);
        let effective_input_border = date_picker_theme
            .input_decoration_theme
            .as_ref()
            .and_then(|theme| theme.border.clone())
            .or_else(|| input_theme.border.clone())
            .unwrap_or_else(|| {
                if use_material3 {
                    InputBorderRef::new(OutlineInputBorder::new())
                } else {
                    InputBorderRef::new(UnderlineInputBorder::new())
                }
            });

        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let hint_text = widget
            .field_hint_text
            .clone()
            .unwrap_or_else(|| delegate.date_help_text(&*localizations));
        let label_text = widget
            .field_label_text
            .clone()
            .unwrap_or_else(|| localizations.date_input_label());
        let keyboard_type = widget.keyboard_type.unwrap_or(TextInputType::DATETIME);
        let autofocus = widget.autofocus;
        let focus_node = widget.focus_node.clone();
        let input_formatters = delegate.keyboard_input_formatters(&*localizations);
        let controller = self.controller(app);

        let mut merged = input_theme.merge(date_picker_theme.input_decoration_theme.as_ref());
        merged.border = Some(effective_input_border);
        let decoration = InputDecoration::new()
            .hint_text(hint_text)
            .label_text(label_text)
            .apply_defaults(merged);

        let mut field = TextFormField::new()
            .decoration(Some(decoration))
            .validator(move |app, text| self.validate_date(app, text))
            .keyboard_type(keyboard_type)
            .on_saved(move |app, text| self.handle_saved(app, text))
            .on_field_submitted(move |app, text| self.handle_submitted(app, text))
            .autofocus(autofocus)
            .controller(controller.erase());
        if let Some(focus_node) = focus_node {
            field = field.focus_node(focus_node);
        }
        if let Some(input_formatters) = input_formatters {
            field = field.input_formatters(input_formatters);
        }
        Semantics::new().container(true).child(field).into_widget()
    }
}

impl fmt::Debug for InputDatePickerFormFieldState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputDatePickerFormFieldState")
            .field("selected_date", &self.selected_date)
            .field("input_text", &self.input_text)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::local_date;
    use std::cell::Cell;

    use waft_embedder::{TargetPlatform, TextAffinity};
    use waft_services::TextInputClient;
    use waft_widgets::{EditableText, EditableTextState, FocusNode, Form, FormState, GlobalKey};

    use waft_widgets::testing::{WidgetTester, expect, find, finds_one_widget, test_widgets};

    use super::*;
    use crate::date::{CalendarDelegate, GregorianCalendarDelegate};
    use crate::input_decorator::InputDecorationThemeData;
    use crate::material::Material;
    use crate::material_localizations::MaterialLocalizations;
    use crate::test_support::{PaintInspection, material_app};
    use crate::text_field::TextField;
    use crate::theme_data::ThemeData;
    use waft_rendering::testing::{PaintCall, PaintedStyle, assert_paints, paints};

    /// The options of Dart's test helper `inputDatePickerField(..)`.
    #[derive(Clone, Default)]
    struct FieldOptions {
        initial_date: Option<DateTime>,
        first_date: Option<DateTime>,
        last_date: Option<DateTime>,
        on_date_saved: Option<ValueChanged<DateTime>>,
        selectable_day_predicate: Option<SelectableDayPredicate>,
        error_format_text: Option<String>,
        error_invalid_text: Option<String>,
        accept_empty_date: bool,
        form_key: Option<GlobalKey>,
        focus_node: Option<FocusNodeRef>,
        field_hint_text: Option<String>,
        field_label_text: Option<String>,
        theme: Option<ThemeData>,
    }

    fn input_date_picker_field(tester: &WidgetTester, options: FieldOptions) -> WidgetRef {
        let mut field = InputDatePickerFormField::new(
            options.first_date.unwrap_or(local_date(2001, 1, 1)),
            options.last_date.unwrap_or(local_date(2031, 12, 31)),
        )
        .initial_date(options.initial_date.unwrap_or(local_date(2016, 1, 15)))
        .accept_empty_date(options.accept_empty_date);
        if let Some(on_date_saved) = options.on_date_saved {
            field = field.on_date_saved(on_date_saved);
        }
        if let Some(predicate) = options.selectable_day_predicate {
            field = field.selectable_day_predicate(predicate);
        }
        if let Some(text) = options.error_format_text {
            field = field.error_format_text(text);
        }
        if let Some(text) = options.error_invalid_text {
            field = field.error_invalid_text(text);
        }
        if let Some(focus_node) = options.focus_node {
            field = field.focus_node(focus_node);
        }
        if let Some(text) = options.field_hint_text {
            field = field.field_hint_text(text);
        }
        if let Some(text) = options.field_label_text {
            field = field.field_label_text(text);
        }
        let mut form = Form::new(field);
        if let Some(form_key) = options.form_key {
            form = form.key(form_key.into());
        }
        let theme = options
            .theme
            .unwrap_or_else(|| crate::tooltip::tests::m3(&tester.cell.borrow()));
        material_app(theme, Material::new().child(form).into_widget())
    }

    fn pump(tester: &mut WidgetTester, options: FieldOptions) {
        let root = input_date_picker_field(tester, options);
        tester.pump_widget(root);
    }

    fn text_field_element(tester: &WidgetTester) -> waft_widgets::AnyElement {
        tester.element(find::by_type::<TextField>().at(0))
    }

    /// Dart's `textFieldController(tester).value.text`.
    fn controller_text(tester: &WidgetTester) -> String {
        let controller = tester.widget(text_field_element(tester), |field: &TextField| {
            field.controller.clone().expect("a controller")
        });
        controller.text_value(&tester.cell.borrow()).to_owned()
    }

    /// Dart's `textFieldController(tester).text = text`.
    fn set_controller_text(tester: &mut WidgetTester, text: &str) {
        let controller = tester.widget(text_field_element(tester), |field: &TextField| {
            field.controller.clone().expect("a controller")
        });
        controller.set_text(&mut tester.cell.borrow_mut(), text.to_owned());
        tester.cell.checkpoint();
    }

    /// Dart's `tester.enterText(find.byType(TextField), text)`.
    fn enter_text(tester: &mut WidgetTester, text: &str) {
        let editable = tester.element(find::by_type::<EditableText>().at(0));
        let state = editable
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state");
        state.request_keyboard(&mut tester.cell.borrow_mut());
        tester.pump();
        let length = text.encode_utf16().count() as i32;
        TextInputClient::update_editing_value(
            state,
            &mut tester.cell.borrow_mut(),
            TextEditingValue::new()
                .text(text)
                .selection(TextSelection::collapsed(length, TextAffinity::Downstream)),
        );
        tester.cell.checkpoint();
    }

    fn form_save(tester: &mut WidgetTester, form_key: &GlobalKey) {
        let mut app = tester.cell.borrow_mut();
        let form = form_key
            .current_state::<FormState>(&app)
            .expect("the form mounted");
        form.save(&mut app);
    }

    fn form_validate(tester: &mut WidgetTester, form_key: &GlobalKey) -> bool {
        let mut app = tester.cell.borrow_mut();
        let form = form_key
            .current_state::<FormState>(&app)
            .expect("the form mounted");
        form.validate(&mut app)
    }

    fn has_text(tester: &WidgetTester, text: &str) -> bool {
        !find::text(text).evaluate(tester).is_empty()
    }

    fn saved() -> (Rc<Cell<Option<DateTime>>>, ValueChanged<DateTime>) {
        let input_date = Rc::new(Cell::new(None));
        let on_date_saved: ValueChanged<DateTime> = {
            let input_date = Rc::clone(&input_date);
            Rc::new(move |_app, date| input_date.set(Some(date)))
        };
        (input_date, on_date_saved)
    }

    #[test]
    fn initial_date_is_the_default() {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            let initial_date = local_date(2016, 2, 21);
            let (input_date, on_date_saved) = saved();
            pump(
                tester,
                FieldOptions {
                    initial_date: Some(initial_date),
                    on_date_saved: Some(on_date_saved),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );
            assert_eq!(controller_text(tester), "02/21/2016");
            form_save(tester, &form_key);
            assert_eq!(input_date.get(), Some(initial_date));
        });
    }

    #[test]
    fn changing_initial_date_is_reflected_in_text_value() {
        test_widgets(|tester| {
            pump(
                tester,
                FieldOptions {
                    initial_date: Some(local_date(2016, 2, 21)),
                    ..FieldOptions::default()
                },
            );
            assert_eq!(controller_text(tester), "02/21/2016");

            pump(
                tester,
                FieldOptions {
                    initial_date: Some(local_date(2016, 2, 23)),
                    ..FieldOptions::default()
                },
            );
            tester.pump_and_settle();
            assert_eq!(controller_text(tester), "02/23/2016");
        });
    }

    #[test]
    fn valid_date_entry() {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            let (input_date, on_date_saved) = saved();
            pump(
                tester,
                FieldOptions {
                    on_date_saved: Some(on_date_saved),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );

            set_controller_text(tester, "02/21/2016");
            form_save(tester, &form_key);
            assert_eq!(input_date.get(), Some(local_date(2016, 2, 21)));
        });
    }

    #[test]
    fn invalid_text_entry_shows_error_format_text() {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            let (input_date, on_date_saved) = saved();
            pump(
                tester,
                FieldOptions {
                    on_date_saved: Some(Rc::clone(&on_date_saved)),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );
            // Default errorFormat text
            assert!(!has_text(tester, "Invalid format."));
            enter_text(tester, "foobar");
            assert!(!form_validate(tester, &form_key));
            tester.pump_and_settle();
            assert_eq!(input_date.get(), None);
            expect(tester, &find::text("Invalid format."), finds_one_widget());

            // Change to a custom errorFormat text
            pump(
                tester,
                FieldOptions {
                    on_date_saved: Some(on_date_saved),
                    error_format_text: Some("That is not a date.".to_owned()),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );
            assert!(!form_validate(tester, &form_key));
            tester.pump_and_settle();
            assert!(!has_text(tester, "Invalid format."));
            expect(
                tester,
                &find::text("That is not a date."),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn valid_text_entry_but_date_outside_first_or_last_date_shows_error_invalid_text() {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            let (input_date, on_date_saved) = saved();
            pump(
                tester,
                FieldOptions {
                    first_date: Some(local_date(1966, 2, 21)),
                    last_date: Some(local_date(2040, 2, 23)),
                    on_date_saved: Some(Rc::clone(&on_date_saved)),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );
            // Default errorInvalid text
            assert!(!has_text(tester, "Out of range."));
            // Before first date
            enter_text(tester, "02/21/1950");
            assert!(!form_validate(tester, &form_key));
            tester.pump_and_settle();
            assert_eq!(input_date.get(), None);
            expect(tester, &find::text("Out of range."), finds_one_widget());
            // After last date
            enter_text(tester, "02/23/2050");
            assert!(!form_validate(tester, &form_key));
            tester.pump_and_settle();
            assert_eq!(input_date.get(), None);
            expect(tester, &find::text("Out of range."), finds_one_widget());

            pump(
                tester,
                FieldOptions {
                    on_date_saved: Some(on_date_saved),
                    error_invalid_text: Some("Not in given range.".to_owned()),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );
            assert!(!form_validate(tester, &form_key));
            tester.pump_and_settle();
            assert!(!has_text(tester, "Out of range."));
            expect(
                tester,
                &find::text("Not in given range."),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn selectable_day_predicate_will_be_used_to_show_error_invalid_if_date_is_not_selectable() {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            let (input_date, on_date_saved) = saved();
            pump(
                tester,
                FieldOptions {
                    initial_date: Some(local_date(2016, 1, 16)),
                    on_date_saved: Some(on_date_saved),
                    selectable_day_predicate: Some(Rc::new(|app: &App, date: DateTime| {
                        date.day(app) % 2 == 0
                    })),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );
            // Default errorInvalid text
            assert!(!has_text(tester, "Out of range."));
            // Odd day shouldn't be valid
            enter_text(tester, "02/21/1966");
            assert!(!form_validate(tester, &form_key));
            tester.pump_and_settle();
            assert_eq!(input_date.get(), None);
            expect(tester, &find::text("Out of range."), finds_one_widget());
            // Even day is valid
            enter_text(tester, "02/24/2030");
            assert!(form_validate(tester, &form_key));
            form_save(tester, &form_key);
            tester.pump_and_settle();
            assert_eq!(input_date.get(), Some(local_date(2030, 2, 24)));
            assert!(!has_text(tester, "Out of range."));
        });
    }

    #[test]
    fn when_an_empty_date_is_entered_and_accept_empty_date_is_true_error_format_text_is_not_shown()
    {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            pump(
                tester,
                FieldOptions {
                    error_format_text: Some("That is not a date.".to_owned()),
                    form_key: Some(form_key.clone()),
                    accept_empty_date: true,
                    ..FieldOptions::default()
                },
            );
            enter_text(tester, "");
            tester.pump_and_settle();
            form_validate(tester, &form_key);
            tester.pump_and_settle();
            assert!(!has_text(tester, "That is not a date."));
        });
    }

    #[test]
    fn when_an_empty_date_is_entered_and_accept_empty_date_is_false_error_format_text_is_shown() {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            pump(
                tester,
                FieldOptions {
                    error_format_text: Some("That is not a date.".to_owned()),
                    form_key: Some(form_key.clone()),
                    ..FieldOptions::default()
                },
            );
            enter_text(tester, "");
            tester.pump_and_settle();
            form_validate(tester, &form_key);
            tester.pump_and_settle();
            expect(
                tester,
                &find::text("That is not a date."),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn focus_node_can_request_focus() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump(
                tester,
                FieldOptions {
                    focus_node: Some(focus_node.clone().into_erased()),
                    ..FieldOptions::default()
                },
            );
            let has_focus = |tester: &WidgetTester| {
                use waft_widgets::FocusNodeLeaf;
                focus_node.handle().has_focus(&mut tester.cell.borrow_mut())
            };
            assert!(!has_focus(tester));
            {
                use waft_widgets::FocusNodeLeaf;
                focus_node
                    .handle()
                    .request_focus(&mut tester.cell.borrow_mut(), None);
            }
            tester.pump_and_settle();
            assert!(has_focus(tester));
            {
                use waft_widgets::FocusNodeLeaf;
                focus_node
                    .handle()
                    .as_node()
                    .unfocus(&mut tester.cell.borrow_mut(), Default::default());
            }
            tester.pump_and_settle();
            assert!(!has_focus(tester));
        });
    }

    #[test]
    fn input_date_picker_form_field_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let field =
                InputDatePickerFormField::new(local_date(2001, 1, 1), local_date(2031, 12, 31))
                    .initial_date(local_date(2016, 1, 15));
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(Form::new(waft_widgets::SizedBox::shrink().child(field)))
                    .into_widget(),
            ));
            let _ = TargetPlatform::Android;
        });
    }

    /// Dart's `textOpacity(tester, text)`: the value of the first `FadeTransition` above `text`.
    fn text_opacity(tester: &WidgetTester, text: &str) -> f64 {
        let element = tester.element(find::text(text).first());
        let fade = tester
            .ancestors(element)
            .into_iter()
            .find(|ancestor| {
                let app = tester.cell.borrow();
                waft_widgets::downcast_widget::<waft_widgets::FadeTransition>(
                    &**ancestor.widget(&app),
                )
                .is_some()
            })
            .expect("a FadeTransition above the text");
        let opacity = tester.widget(fade, |fade: &waft_widgets::FadeTransition| {
            fade.opacity.clone()
        });
        opacity.value(&tester.cell.borrow())
    }

    #[test]
    fn empty_field_shows_hint_text_when_focused() {
        test_widgets(|tester| {
            pump(tester, FieldOptions::default());
            // Focus on it
            let at = crate::chip::tests::center_of(tester, text_field_element(tester));
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            // Hint text should be invisible
            assert_eq!(text_opacity(tester, "mm/dd/yyyy"), 0.0);
            set_controller_text(tester, "");
            tester.pump_and_settle();
            // Hint text should be visible
            assert_eq!(text_opacity(tester, "mm/dd/yyyy"), 1.0);

            // Change to a different hint text
            pump(
                tester,
                FieldOptions {
                    field_hint_text: Some("Enter some date".to_owned()),
                    ..FieldOptions::default()
                },
            );
            tester.pump_and_settle();
            assert!(!has_text(tester, "mm/dd/yyyy"));
            assert_eq!(text_opacity(tester, "Enter some date"), 1.0);
            enter_text(tester, "foobar");
            tester.pump_and_settle();
            assert_eq!(text_opacity(tester, "Enter some date"), 0.0);
        });
    }

    #[test]
    fn label_text() {
        test_widgets(|tester| {
            pump(tester, FieldOptions::default());
            // Default label
            expect(tester, &find::text("Enter Date"), finds_one_widget());

            pump(
                tester,
                FieldOptions {
                    field_label_text: Some("Give me a date!".to_owned()),
                    ..FieldOptions::default()
                },
            );
            assert!(!has_text(tester, "Enter Date"));
            expect(tester, &find::text("Give me a date!"), finds_one_widget());
        });
    }

    /// What the text field's subtree paints: Dart reads the `_InputBorderPainter` of the
    /// `_BorderContainer`, whose border is what `InputDecorator` draws.
    fn text_field_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
        tester.painted_ops_deep(tester.render_object(text_field_element(tester)))
    }

    /// The `drawPath` of a stroked border.
    fn stroked_border() -> PaintCall {
        PaintCall::Path {
            color: None,
            style: Some(PaintedStyle::Stroke(None)),
            includes: Vec::new(),
            excludes: Vec::new(),
        }
    }

    #[test]
    fn theme_data_input_decoration_theme_is_honored() {
        test_widgets(|tester| {
            // The default theme draws the text field's outline.
            pump(tester, FieldOptions::default());
            tester.pump_and_settle();
            assert_paints(&text_field_ops(tester), &[stroked_border()]);

            let theme = ThemeData::builder()
                .input_decoration_theme(
                    InputDecorationThemeData::new().border(InputBorderRef::none()),
                )
                .build(&tester.cell.borrow());
            pump(
                tester,
                FieldOptions {
                    theme: Some(theme),
                    ..FieldOptions::default()
                },
            );
            tester.pump_and_settle();
            // Border should match: none is drawn.
            assert!(paints(&text_field_ops(tester), &[stroked_border()]).is_err());
        });
    }

    #[test]
    fn local_input_decoration_theme_is_honored() {
        test_widgets(|tester| {
            let field =
                InputDatePickerFormField::new(local_date(2025, 2, 1), local_date(2026, 5, 1));
            let theme = crate::tooltip::tests::m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        waft_widgets::Center::new().child(
                            crate::input_decorator::InputDecorationTheme::new()
                                .with_data(InputDecorationThemeData::new().filled(true))
                                .child(field),
                        ),
                    )
                    .into_widget(),
            ));

            let decoration = tester.widget(text_field_element(tester), |field: &TextField| {
                field.decoration.clone().expect("a decoration")
            });
            assert_eq!(decoration.filled, Some(true));
        });
    }

    /// Dart's `TestCalendarDelegate extends GregorianCalendarDelegate`, overriding how a date is
    /// formatted and parsed; the rest forwards to the Gregorian delegate.
    #[derive(Debug)]
    struct TestCalendarDelegate;

    impl CalendarDelegate for TestCalendarDelegate {
        fn now(&self, app: &App) -> DateTime {
            GregorianCalendarDelegate.now(app)
        }
        fn date_only(&self, app: &App, date: &DateTime) -> DateTime {
            GregorianCalendarDelegate.date_only(app, date)
        }
        fn month_delta(&self, app: &App, start_date: &DateTime, end_date: &DateTime) -> i32 {
            GregorianCalendarDelegate.month_delta(app, start_date, end_date)
        }
        fn add_months_to_month_date(
            &self,
            app: &App,
            month_date: &DateTime,
            months_to_add: i32,
        ) -> DateTime {
            GregorianCalendarDelegate.add_months_to_month_date(app, month_date, months_to_add)
        }
        fn add_days_to_date(&self, app: &App, date: &DateTime, days: i32) -> DateTime {
            GregorianCalendarDelegate.add_days_to_date(app, date, days)
        }
        fn first_day_offset(
            &self,
            app: &App,
            year: i32,
            month: i32,
            localizations: &dyn MaterialLocalizations,
        ) -> i32 {
            GregorianCalendarDelegate.first_day_offset(app, year, month, localizations)
        }
        fn get_days_in_month(&self, year: i32, month: i32) -> i32 {
            GregorianCalendarDelegate.get_days_in_month(year, month)
        }
        fn get_month(&self, app: &App, year: i32, month: i32) -> DateTime {
            GregorianCalendarDelegate.get_month(app, year, month)
        }
        fn get_day(&self, app: &App, year: i32, month: i32, day: i32) -> DateTime {
            GregorianCalendarDelegate.get_day(app, year, month, day)
        }
        fn format_month_year(
            &self,
            app: &App,
            date: &DateTime,
            localizations: &dyn MaterialLocalizations,
        ) -> String {
            GregorianCalendarDelegate.format_month_year(app, date, localizations)
        }
        fn format_medium_date(
            &self,
            app: &App,
            date: &DateTime,
            localizations: &dyn MaterialLocalizations,
        ) -> String {
            GregorianCalendarDelegate.format_medium_date(app, date, localizations)
        }
        fn format_short_month_day(
            &self,
            app: &App,
            date: &DateTime,
            localizations: &dyn MaterialLocalizations,
        ) -> String {
            GregorianCalendarDelegate.format_short_month_day(app, date, localizations)
        }
        fn format_short_date(
            &self,
            app: &App,
            date: &DateTime,
            localizations: &dyn MaterialLocalizations,
        ) -> String {
            GregorianCalendarDelegate.format_short_date(app, date, localizations)
        }
        fn format_full_date(
            &self,
            app: &App,
            date: &DateTime,
            localizations: &dyn MaterialLocalizations,
        ) -> String {
            GregorianCalendarDelegate.format_full_date(app, date, localizations)
        }
        fn format_compact_date(
            &self,
            app: &App,
            date: &DateTime,
            _localizations: &dyn MaterialLocalizations,
        ) -> String {
            format!("{}..{}..{}", date.year(app), date.month(app), date.day(app))
        }
        fn parse_compact_date(
            &self,
            app: &App,
            input_string: Option<&str>,
            _localizations: &dyn MaterialLocalizations,
        ) -> Option<DateTime> {
            let parts: Vec<&str> = input_string.expect("an input string").split("..").collect();
            if parts.len() != 3 {
                return None;
            }
            let number = |part: &str| part.parse::<i32>().unwrap_or(0);
            Some(DateTime::new(
                app,
                number(parts[0]),
                number(parts[1]),
                number(parts[2]),
            ))
        }
        fn date_help_text(&self, _localizations: &dyn MaterialLocalizations) -> String {
            "yyyy..mm..dd".to_owned()
        }
    }

    #[test]
    fn displays_calendar_based_on_the_calendar_delegate() {
        test_widgets(|tester| {
            let selected_date = Rc::new(Cell::new(None));
            let field =
                InputDatePickerFormField::new(local_date(2025, 2, 1), local_date(2026, 5, 1))
                    .initial_date(local_date(2025, 2, 26))
                    .on_date_submitted({
                        let selected_date = Rc::clone(&selected_date);
                        Rc::new(move |_app, date| selected_date.set(Some(date)))
                    })
                    .calendar_delegate(Rc::new(TestCalendarDelegate));
            let theme = crate::tooltip::tests::m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new().child(field).into_widget(),
            ));

            assert_eq!(controller_text(tester), "2025..2..26");
            enter_text(tester, "2025..3..10");
            receive_done(tester);
            tester.pump_and_settle();
            assert_eq!(selected_date.get(), Some(local_date(2025, 3, 10)));
            assert_eq!(controller_text(tester), "2025..3..10");

            enter_text(tester, "2025..4..21");
            receive_done(tester);
            tester.pump_and_settle();
            assert_eq!(selected_date.get(), Some(local_date(2025, 4, 21)));
        });
    }

    /// Dart's `testTextInput.receiveAction(TextInputAction.done)`.
    fn receive_done(tester: &mut WidgetTester) {
        let editable = tester.element(find::by_type::<EditableText>().at(0));
        let state = editable
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state");
        TextInputClient::perform_action(
            state,
            &mut tester.cell.borrow_mut(),
            waft_services::TextInputAction::Done,
        );
        tester.cell.checkpoint();
    }
}
