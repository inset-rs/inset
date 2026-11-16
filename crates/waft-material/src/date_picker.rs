//! Flutter counterpart: material_ui's `date_picker.dart`.
//!
//! The date picker dialogs and the functions that show them: [`show_date_picker`] with
//! [`DatePickerDialog`], and [`show_date_range_picker`] with [`DateRangePickerDialog`].

use crate::action_buttons::CloseButton;
use crate::app_bar::AppBar;
use crate::calendar_date_picker::{
    CalendarDatePicker, CalendarDatePickerDayBuilder, CalendarDatePickerWeekdayBuilder,
};
use crate::date::{
    CalendarDelegate, DatePickerEntryMode, DatePickerMode, DateTimeRange, GregorianCalendarDelegate,
};
use crate::date_picker_theme::DatePickerTheme;
use crate::dialog::Dialog;
use crate::divider::{Divider, VerticalDivider};
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::ink_well::InkResponse;
use crate::input_border::{InputBorderRef, OutlineInputBorder, UnderlineInputBorder};
use crate::input_date_picker_form_field::InputDatePickerFormField;
use crate::input_decorator::{InputDecoration, InputDecorationTheme};
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::scaffold::Scaffold;
use crate::text_button::TextButton;
use crate::text_field::TextField;
use crate::theme::Theme;
use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;
use waft_animation::Curves;
use waft_embedder::{
    Canvas, Clip, Color, Locale, Paint, Rect, Size, TargetPlatform, TextSelection,
};
use waft_foundation::{
    App, CompleterFuture, DateTime, Handle, ListenableObject, Listener, Ref, UniqueKey,
    ValueChanged,
};
use waft_painting::{
    Alignment, AlignmentDirectional, AlignmentGeometry, BorderSide, CircleBorder, EdgeInsets,
    EdgeInsetsDirectional, OutlinedBorderRef, ShapeBorderRef, ShapeDecoration, TextOverflow,
    TextStyle, axis_direction_is_reversed,
};
use waft_rendering::{
    BoxConstraints, CrossAxisAlignment, CustomPainter, MainAxisAlignment, MainAxisSize,
    SliverConstraints, SliverGridDelegate, SliverGridGeometry, SliverGridLayout,
};
use waft_services::{HapticFeedback, LogicalKeyboardKey, TextInputType};
use waft_widgets::{
    ActionRef, Align, AnimatedContainer, AutovalidateMode, BuildContext, Builder, CallbackAction,
    Center, ColoredBox, Column, ConstrainedBox, Container, CustomPaint, CustomScrollView,
    DirectionalFocusIntent, Directionality, ExcludeSemantics, Expanded, Flexible, FocusNode,
    FocusNodeLeaf, FocusableActionDetector, Form, FormState, GlobalKey, GridView, Icon, IconTheme,
    IconThemeData, InheritedWidget, IntentRef, IntoWidget, KeyRef, LayoutBuilder, LimitedBox,
    Localizations, MediaQuery, Navigator, NextFocusIntent, Orientation, OverflowBar, Padding,
    PreferredSize, PreferredSizeWidgetRef, PreviousFocusIntent, RestorableBool,
    RestorableDateTimeN, RestorableEnum, RestorableProperty, RestorableValue, RestorationMixin,
    RestorationMixinData, RouteResult, RouteSettingsRef, Row, SafeArea, ScrollController,
    ScrollControllerLeaf, ScrollPositionAlignmentPolicy, Scrollable, SelectableDayPredicate,
    Semantics, ShortcutActivatorRef, ShortcutMap, Shortcuts, SingleActivator, SizedBox,
    SliverChildListDelegate, SliverList, Spacer, State, StateData, StatefulWidget, StatelessWidget,
    Text, TextDirection, TextEditingController, TextEditingControllerLeaf, TransitionBuilder,
    TraversalDirection, WidgetRef, WidgetStateProperty, WidgetStates, WidgetStatesController,
};

const CALENDAR_PORTRAIT_DIALOG_SIZE_M2: Size = Size::new(330.0, 518.0);
const CALENDAR_PORTRAIT_DIALOG_SIZE_M3: Size = Size::new(360.0, 568.0);
const CALENDAR_LANDSCAPE_DIALOG_SIZE: Size = Size::new(496.0, 346.0);
const INPUT_PORTRAIT_DIALOG_SIZE_M2: Size = Size::new(330.0, 270.0);
const INPUT_PORTRAIT_DIALOG_SIZE_M3: Size = Size::new(328.0, 270.0);
const INPUT_LANDSCAPE_DIALOG_SIZE: Size = Size::new(496.0, 160.0);
const INPUT_RANGE_LANDSCAPE_DIALOG_SIZE: Size = Size::new(496.0, 164.0);
const DIALOG_SIZE_ANIMATION_DURATION: Duration = Duration::from_millis(200);
const INPUT_FORM_PORTRAIT_HEIGHT: f64 = 98.0;
const INPUT_FORM_LANDSCAPE_HEIGHT: f64 = 108.0;

// 3.0 is the maximum scale factor on mobile phones. As of 07/30/24, iOS goes up to a max of
// 3.310 on each of 2 flavors of text scaling, and Android goes up to 2.0 on its text scaling.
const MAX_TEXT_SCALE_FACTOR: f64 = 3.0;

// Max scale factor for the range pickers, whose layout does not have space to grow.
const MAX_RANGE_TEXT_SCALE_FACTOR: f64 = 1.3;

// Max scale factor for the header, which fits in a fixed-height container.
const MAX_HEADER_TEXT_SCALE_FACTOR: f64 = 1.6;

// Max scale factor for the header when it has an entry mode button.
const MAX_HEADER_WITH_ENTRY_TEXT_SCALE_FACTOR: f64 = 1.4;

const MAX_HELP_PORTRAIT_TEXT_SCALE_FACTOR: f64 = 1.6;
const MAX_HELP_LANDSCAPE_TEXT_SCALE_FACTOR: f64 = 1.4;

// 14 is a common font size used to compute the effective text scale.
const FONT_SIZE_TO_SCALE: f64 = 14.0;

/// Shows a dialog containing a Material Design date picker.
///
/// The returned future resolves when the user closes the dialog. If the user cancels the dialog,
/// it resolves with null; otherwise it resolves to the selected [`DateTime`] (inside the
/// [`RouteResult`]).
///
/// An optional `initial_date` tells the picker which date to open on; with none the picker
/// shows the input mode without a date. The `first_date` is the earliest allowable date and the
/// `last_date` the latest. The `initial_date`, if given, must not be outside that range, and
/// must satisfy the `selectable_day_predicate`.
///
/// The `current_date` represents the current day (i.e. today). This date will be highlighted in
/// the day grid. If null, the date of `calendar_delegate.now()` will be used.
///
/// An optional `initial_entry_mode` argument can be used to display the date picker in the
/// [`DatePickerEntryMode::Calendar`] (a calendar month grid) or [`DatePickerEntryMode::Input`]
/// (a text input field) mode. It defaults to [`DatePickerEntryMode::Calendar`].
///
/// An optional `selectable_day_predicate` function can be passed in to customize the days to
/// enable for selection. If provided, only the days that `selectable_day_predicate` returns
/// true for will be selectable.
///
/// The following optional string parameters allow you to override the default text used for
/// various parts of the dialog: `help_text`, `cancel_text`, `confirm_text`, `error_format_text`,
/// `error_invalid_text`, `field_hint_text` and `field_label_text`.
///
/// An optional `locale` argument can be used to set the locale for the date picker. It defaults
/// to the ambient locale provided by [`Localizations`].
///
/// An optional `text_direction` argument can be used to set the text direction
/// ([`TextDirection::Ltr`] or [`TextDirection::Rtl`]) for the date picker. It defaults to the
/// ambient text direction provided by `Directionality`.
///
/// The `builder` parameter can be used to wrap the dialog widget to add inherited widgets like
/// [`Theme`](crate::Theme).
///
/// An optional `initial_date_picker_mode` argument can be used to have the calendar
/// date picker initially appear in the [`DatePickerMode::Year`] or [`DatePickerMode::Day`]
/// mode. It defaults to [`DatePickerMode::Day`].
///
/// The `on_date_picker_mode_change` callback is called when the user toggles the entry mode.
///
/// The `calendar_delegate` provides the calendar the dialog works in; it defaults to the
/// Gregorian calendar.
///
/// Dart's `anchorPoint` waits for `show_dialog` to take one.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_date_picker(
    app: &mut App,
    context: BuildContext,
    initial_date: Option<DateTime>,
    first_date: DateTime,
    last_date: DateTime,
    current_date: Option<DateTime>,
    initial_entry_mode: DatePickerEntryMode,
    selectable_day_predicate: Option<SelectableDayPredicate>,
    help_text: Option<String>,
    cancel_text: Option<String>,
    confirm_text: Option<String>,
    locale: Option<Locale>,
    barrier_dismissible: bool,
    barrier_color: Option<Color>,
    barrier_label: Option<String>,
    use_root_navigator: bool,
    route_settings: Option<RouteSettingsRef>,
    text_direction: Option<TextDirection>,
    builder: Option<TransitionBuilder>,
    initial_date_picker_mode: DatePickerMode,
    error_format_text: Option<String>,
    error_invalid_text: Option<String>,
    field_hint_text: Option<String>,
    field_label_text: Option<String>,
    keyboard_type: Option<TextInputType>,
    on_date_picker_mode_change: Option<ValueChanged<DatePickerEntryMode>>,
    switch_to_input_entry_mode_icon: Option<Icon>,
    switch_to_calendar_entry_mode_icon: Option<Icon>,
    day_builder: Option<CalendarDatePickerDayBuilder>,
    weekday_builder: Option<CalendarDatePickerWeekdayBuilder>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
) -> CompleterFuture<RouteResult> {
    let initial_date = initial_date.map(|date| calendar_delegate.date_only(&date));
    let first_date = calendar_delegate.date_only(&first_date);
    let last_date = calendar_delegate.date_only(&last_date);
    assert!(
        !last_date.is_before(&first_date),
        "lastDate {last_date} must be on or after firstDate {first_date}."
    );
    assert!(
        initial_date.is_none_or(|date| !date.is_before(&first_date)),
        "initialDate {initial_date:?} must be on or after firstDate {first_date}."
    );
    assert!(
        initial_date.is_none_or(|date| !date.is_after(&last_date)),
        "initialDate {initial_date:?} must be on or before lastDate {last_date}."
    );
    assert!(
        selectable_day_predicate
            .as_ref()
            .zip(initial_date)
            .is_none_or(|(predicate, date)| predicate(date)),
        "Provided initialDate {initial_date:?} must satisfy provided selectableDayPredicate."
    );
    debug_assert!(crate::debug::debug_check_has_material_localizations(
        app, context
    ));

    let mut dialog = DatePickerDialog::new(first_date, last_date)
        .initial_entry_mode(initial_entry_mode)
        .initial_calendar_mode(initial_date_picker_mode)
        .calendar_delegate(Rc::clone(&calendar_delegate));
    dialog.initial_date = initial_date;
    dialog.current_date =
        calendar_delegate.date_only(&current_date.unwrap_or_else(|| calendar_delegate.now()));
    dialog.selectable_day_predicate = selectable_day_predicate;
    dialog.help_text = help_text;
    dialog.cancel_text = cancel_text;
    dialog.confirm_text = confirm_text;
    dialog.error_format_text = error_format_text;
    dialog.error_invalid_text = error_invalid_text;
    dialog.field_hint_text = field_hint_text;
    dialog.field_label_text = field_label_text;
    dialog.keyboard_type = keyboard_type;
    dialog.on_date_picker_mode_change = on_date_picker_mode_change;
    dialog.switch_to_input_entry_mode_icon =
        switch_to_input_entry_mode_icon.map(IntoWidget::into_widget);
    dialog.switch_to_calendar_entry_mode_icon =
        switch_to_calendar_entry_mode_icon.map(IntoWidget::into_widget);
    dialog.day_builder = day_builder;
    dialog.weekday_builder = weekday_builder;
    let mut dialog = dialog.into_widget();

    if let Some(text_direction) = text_direction {
        dialog = Directionality::new(text_direction, dialog).into_widget();
    }

    if let Some(locale) = locale {
        dialog = Localizations::r#override(app, context)
            .locale(locale)
            .child(dialog)
            .into_widget();
    } else if let Some(locale) = DatePickerTheme::of(app, context).locale {
        dialog = Localizations::r#override(app, context)
            .locale(locale)
            .child(dialog)
            .into_widget();
    }

    crate::dialog::show_dialog(
        app,
        context,
        Rc::new(move |app, context| match &builder {
            None => dialog.clone(),
            Some(builder) => builder(app, context, Some(&dialog)),
        }),
        barrier_dismissible,
        barrier_color,
        barrier_label,
        true,
        use_root_navigator,
        route_settings,
        None,
        false,
        None,
        None,
    )
}

const ENTRY_MODES: [DatePickerEntryMode; 4] = [
    DatePickerEntryMode::Calendar,
    DatePickerEntryMode::Input,
    DatePickerEntryMode::CalendarOnly,
    DatePickerEntryMode::InputOnly,
];
const AUTOVALIDATE_MODES: [AutovalidateMode; 4] = [
    AutovalidateMode::Disabled,
    AutovalidateMode::Always,
    AutovalidateMode::OnUserInteraction,
    AutovalidateMode::OnUnfocus,
];

/// A Material-style date picker dialog.
///
/// It displays a calendar grid of days, allowing the user to select a date, and a button to
/// switch to a text field where a date can be typed.
///
/// It is used by [`show_date_picker`](crate::show_date_picker), which should usually be called
/// rather than creating a [`DatePickerDialog`] directly.
///
/// See also: [`CalendarDatePicker`], [`InputDatePickerFormField`], [`show_date_picker`],
/// [`DateRangePickerDialog`](crate::DateRangePickerDialog).
///
/// [`show_date_picker`]: crate::show_date_picker
#[derive(Clone)]
pub struct DatePickerDialog {
    pub key: Option<waft_widgets::KeyRef>,

    /// The initially selected [`DateTime`] that the picker should display.
    ///
    /// If this is null, there is no selected date. A date must be selected to submit the dialog.
    pub initial_date: Option<DateTime>,

    /// The earliest allowable [`DateTime`] that the user can select.
    pub first_date: DateTime,

    /// The latest allowable [`DateTime`] that the user can select.
    pub last_date: DateTime,

    /// The [`DateTime`] representing today. It will be highlighted in the day grid.
    pub current_date: DateTime,

    /// The initial mode of date entry method for the date picker dialog.
    ///
    /// See [`DatePickerEntryMode`] for the different data entry modes.
    pub initial_entry_mode: DatePickerEntryMode,

    /// Function to provide full control over which [`DateTime`] can be selected.
    pub selectable_day_predicate: Option<SelectableDayPredicate>,

    /// The text that is displayed on the cancel button.
    pub cancel_text: Option<String>,

    /// The text that is displayed on the confirm button.
    pub confirm_text: Option<String>,

    /// The text that is displayed at the top of the header.
    ///
    /// This is used to indicate to the user what they are selecting a date for.
    pub help_text: Option<String>,

    /// The initial display of the calendar picker.
    pub initial_calendar_mode: DatePickerMode,

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
    /// If this is null, it will default to the words representing the date format string. For
    /// example, 'Enter Date' for en_US.
    pub field_label_text: Option<String>,

    /// The keyboard type of the [`TextField`](crate::TextField).
    ///
    /// If this is null, it will default to [`TextInputType::datetime`].
    pub keyboard_type: Option<TextInputType>,

    /// Restoration ID to save and restore the state of the [`DatePickerDialog`].
    ///
    /// If it is non-null, the date picker will persist and restore the date selected on the
    /// dialog.
    ///
    /// The state of this widget is persisted in a `RestorationBucket` claimed from the
    /// surrounding `RestorationScope` using the provided restoration ID.
    pub restoration_id: Option<String>,

    /// Called when the [`DatePickerDialog`] is toggled between
    /// [`DatePickerEntryMode::Calendar`] and [`DatePickerEntryMode::Input`].
    pub on_date_picker_mode_change: Option<ValueChanged<DatePickerEntryMode>>,

    /// An optional icon that, when pressed, switches from the calendar to the text input.
    pub switch_to_input_entry_mode_icon: Option<WidgetRef>,

    /// An optional icon that, when pressed, switches from the text input to the calendar.
    pub switch_to_calendar_entry_mode_icon: Option<WidgetRef>,

    /// The padding that surrounds the dialog.
    pub inset_padding: EdgeInsets,

    /// Called to construct the widget for a day in the calendar grid.
    pub day_builder: Option<CalendarDatePickerDayBuilder>,

    /// Called to construct the widget for a weekday header.
    pub weekday_builder: Option<CalendarDatePickerWeekdayBuilder>,

    /// The calendar the dialog works in.
    pub calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl DatePickerDialog {
    /// Creates a [`DatePickerDialog`]. Dart's optional named arguments are the fluent setters.
    ///
    /// [`initial_date`](Self::initial_date), [`first_date`](Self::first_date),
    /// [`last_date`](Self::last_date) and [`current_date`](Self::current_date) are truncated to
    /// the date by the calendar delegate; set the delegate first.
    pub fn new(first_date: DateTime, last_date: DateTime) -> DatePickerDialog {
        let calendar_delegate: Rc<dyn CalendarDelegate> = Rc::new(GregorianCalendarDelegate);
        let dialog = DatePickerDialog {
            key: None,
            initial_date: None,
            first_date: calendar_delegate.date_only(&first_date),
            last_date: calendar_delegate.date_only(&last_date),
            current_date: calendar_delegate.date_only(&calendar_delegate.now()),
            initial_entry_mode: DatePickerEntryMode::Calendar,
            selectable_day_predicate: None,
            cancel_text: None,
            confirm_text: None,
            help_text: None,
            initial_calendar_mode: DatePickerMode::Day,
            error_format_text: None,
            error_invalid_text: None,
            field_hint_text: None,
            field_label_text: None,
            keyboard_type: None,
            restoration_id: None,
            on_date_picker_mode_change: None,
            switch_to_input_entry_mode_icon: None,
            switch_to_calendar_entry_mode_icon: None,
            inset_padding: EdgeInsets::symmetric(24.0, 16.0),
            day_builder: None,
            weekday_builder: None,
            calendar_delegate,
        };
        dialog.debug_check_dates();
        dialog
    }

    fn debug_check_dates(&self) {
        debug_assert!(
            !self.last_date.is_before(&self.first_date),
            "lastDate {} must be on or after firstDate {}.",
            self.last_date,
            self.first_date
        );
        debug_assert!(
            self.initial_date
                .is_none_or(|date| !date.is_before(&self.first_date)),
            "initialDate {:?} must be on or after firstDate {}.",
            self.initial_date,
            self.first_date
        );
        debug_assert!(
            self.initial_date
                .is_none_or(|date| !date.is_after(&self.last_date)),
            "initialDate {:?} must be on or before lastDate {}.",
            self.initial_date,
            self.last_date
        );
        debug_assert!(
            self.selectable_day_predicate
                .as_ref()
                .zip(self.initial_date)
                .is_none_or(|(predicate, date)| predicate(date)),
            "Provided initialDate {:?} must satisfy provided selectableDayPredicate",
            self.initial_date
        );
    }

    /// Dart `DatePickerDialog(key:)`.
    pub fn key(mut self, key: waft_widgets::KeyRef) -> DatePickerDialog {
        self.key = Some(key);
        self
    }

    /// Dart `DatePickerDialog(initialDate:)`.
    pub fn initial_date(mut self, initial_date: DateTime) -> DatePickerDialog {
        self.initial_date = Some(self.calendar_delegate.date_only(&initial_date));
        self
    }

    /// Dart `DatePickerDialog(currentDate:)`.
    pub fn current_date(mut self, current_date: DateTime) -> DatePickerDialog {
        self.current_date = self.calendar_delegate.date_only(&current_date);
        self
    }

    /// Dart `DatePickerDialog(initialEntryMode:)`.
    pub fn initial_entry_mode(
        mut self,
        initial_entry_mode: DatePickerEntryMode,
    ) -> DatePickerDialog {
        self.initial_entry_mode = initial_entry_mode;
        self
    }

    /// Dart `DatePickerDialog(selectableDayPredicate:)`.
    pub fn selectable_day_predicate(
        mut self,
        selectable_day_predicate: SelectableDayPredicate,
    ) -> DatePickerDialog {
        self.selectable_day_predicate = Some(selectable_day_predicate);
        self
    }

    /// Dart `DatePickerDialog(initialCalendarMode:)`.
    pub fn initial_calendar_mode(
        mut self,
        initial_calendar_mode: DatePickerMode,
    ) -> DatePickerDialog {
        self.initial_calendar_mode = initial_calendar_mode;
        self
    }

    /// Dart `DatePickerDialog(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> DatePickerDialog {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `DatePickerDialog(insetPadding:)`.
    pub fn inset_padding(mut self, inset_padding: EdgeInsets) -> DatePickerDialog {
        self.inset_padding = inset_padding;
        self
    }

    /// Dart `DatePickerDialog(calendarDelegate:)`. It truncates the dates already set.
    pub fn calendar_delegate(
        mut self,
        calendar_delegate: Rc<dyn CalendarDelegate>,
    ) -> DatePickerDialog {
        self.initial_date = self
            .initial_date
            .map(|date| calendar_delegate.date_only(&date));
        self.first_date = calendar_delegate.date_only(&self.first_date);
        self.last_date = calendar_delegate.date_only(&self.last_date);
        self.current_date = calendar_delegate.date_only(&self.current_date);
        self.calendar_delegate = calendar_delegate;
        self
    }
}

impl StatefulWidget for DatePickerDialog {
    type State = DatePickerDialogState;

    fn key(&self) -> Option<&waft_widgets::KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DatePickerDialogState {
        DatePickerDialogState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            selected_date: None,
            entry_mode: None,
            autovalidate_mode: None,
            calendar_picker_key: GlobalKey::new(),
            form_key: GlobalKey::new(),
        }
    }
}

/// Dart's `_DatePickerDialogState`.
pub struct DatePickerDialogState {
    state: StateData<DatePickerDialog>,
    restoration: RestorationMixinData,
    selected_date: Option<Ref<RestorableDateTimeN>>,
    entry_mode: Option<Ref<RestorableEnum<DatePickerEntryMode>>>,
    autovalidate_mode: Option<Ref<RestorableEnum<AutovalidateMode>>>,
    calendar_picker_key: GlobalKey,
    form_key: GlobalKey,
}

impl DatePickerDialogState {
    fn selected_date(self: Handle<Self>, app: &App) -> Handle<RestorableDateTimeN> {
        app.get(self)
            .selected_date
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn entry_mode(self: Handle<Self>, app: &App) -> Handle<RestorableEnum<DatePickerEntryMode>> {
        app.get(self)
            .entry_mode
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn autovalidate_mode(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<RestorableEnum<AutovalidateMode>> {
        app.get(self)
            .autovalidate_mode
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn handle_ok(self: Handle<Self>, app: &mut App) {
        let entry_mode = *self.entry_mode(app).value(app);
        if entry_mode == DatePickerEntryMode::Input || entry_mode == DatePickerEntryMode::InputOnly
        {
            let form_key = app.get(self).form_key.clone();
            let form = form_key
                .current_state::<FormState>(app)
                .expect("the form is mounted");
            if !form.validate(app) {
                self.set_state(app, |_state| {});
                self.autovalidate_mode(app)
                    .set_value(app, AutovalidateMode::Always);
                return;
            }
            form.save(app);
        }
        let context = self.context(app);
        let selected_date = *self.selected_date(app).value(app);
        Navigator::pop(
            app,
            context,
            selected_date.map(|date| Rc::new(date) as Rc<dyn std::any::Any>),
        );
    }

    fn handle_cancel(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        Navigator::pop(app, context, None);
    }

    fn handle_on_date_picker_mode_change(self: Handle<Self>, app: &mut App) {
        if let Some(callback) = self.widget(app).on_date_picker_mode_change.clone() {
            let entry_mode = *self.entry_mode(app).value(app);
            callback(app, entry_mode);
        }
    }

    fn handle_entry_mode_toggle(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
        match *self.entry_mode(app).value(app) {
            DatePickerEntryMode::Calendar => {
                self.autovalidate_mode(app)
                    .set_value(app, AutovalidateMode::Disabled);
                self.entry_mode(app)
                    .set_value(app, DatePickerEntryMode::Input);
                self.handle_on_date_picker_mode_change(app);
            }
            DatePickerEntryMode::Input => {
                let form_key = app.get(self).form_key.clone();
                form_key
                    .current_state::<FormState>(app)
                    .expect("the form is mounted")
                    .save(app);
                self.entry_mode(app)
                    .set_value(app, DatePickerEntryMode::Calendar);
                self.handle_on_date_picker_mode_change(app);
            }
            DatePickerEntryMode::CalendarOnly | DatePickerEntryMode::InputOnly => {
                debug_assert!(
                    false,
                    "Can not change entry mode from {:?}",
                    *self.entry_mode(app).value(app)
                );
            }
        }
    }

    fn handle_date_changed(self: Handle<Self>, app: &mut App, date: DateTime) {
        self.set_state(app, |_state| {});
        self.selected_date(app).set_value(app, Some(date));
    }

    fn dialog_size(self: Handle<Self>, app: &mut App, context: BuildContext) -> Size {
        let use_material3 = Theme::of(app, context).use_material3;
        let is_calendar = matches!(
            *self.entry_mode(app).value(app),
            DatePickerEntryMode::Calendar | DatePickerEntryMode::CalendarOnly
        );
        match (is_calendar, MediaQuery::orientation_of(app, context)) {
            (true, Orientation::Portrait) if use_material3 => CALENDAR_PORTRAIT_DIALOG_SIZE_M3,
            (false, Orientation::Portrait) if use_material3 => INPUT_PORTRAIT_DIALOG_SIZE_M3,
            (true, Orientation::Portrait) => CALENDAR_PORTRAIT_DIALOG_SIZE_M2,
            (false, Orientation::Portrait) => INPUT_PORTRAIT_DIALOG_SIZE_M2,
            (true, Orientation::Landscape) => CALENDAR_LANDSCAPE_DIALOG_SIZE,
            (false, Orientation::Landscape) => INPUT_LANDSCAPE_DIALOG_SIZE,
        }
    }

    fn form_shortcut_map() -> waft_widgets::ShortcutMap {
        // Pressing enter on the field will move focus to the next field or control.
        vec![(
            Rc::new(waft_widgets::SingleActivator::new(
                waft_services::LogicalKeyboardKey::ENTER,
            )),
            Rc::new(waft_widgets::NextFocusIntent::new()),
        )]
    }
}

impl RestorationMixin for DatePickerDialogState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<waft_widgets::RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let (selected_date, autovalidate_mode, entry_mode) = (
            self.selected_date(app),
            self.autovalidate_mode(app),
            self.entry_mode(app),
        );
        self.register_for_restoration(app, selected_date.as_property(), "selected_date");
        self.register_for_restoration(app, autovalidate_mode.as_property(), "autovalidateMode");
        self.register_for_restoration(app, entry_mode.as_property(), "calendar_entry_mode");
    }
}

impl State for DatePickerDialogState {
    type Widget = DatePickerDialog;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let selected_date = RestorableDateTimeN::new(app, widget.initial_date);
        let entry_mode = RestorableEnum::new(app, widget.initial_entry_mode, ENTRY_MODES);
        let autovalidate_mode =
            RestorableEnum::new(app, AutovalidateMode::Disabled, AUTOVALIDATE_MODES);
        let state = app.get_mut(self);
        state.selected_date = Some(selected_date);
        state.entry_mode = Some(entry_mode);
        state.autovalidate_mode = Some(autovalidate_mode);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &DatePickerDialog) {
        self.did_update_restoration_id(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.selected_date(app).dispose(app);
        self.entry_mode(app).dispose(app);
        self.autovalidate_mode(app).dispose(app);
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let use_material3 = theme.use_material3;
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let orientation = MediaQuery::orientation_of(app, context);
        let is_landscape_orientation = orientation == Orientation::Landscape;
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let text_theme = theme.text_theme.clone();
        let widget = self.widget(app).clone();
        let entry_mode = *self.entry_mode(app).value(app);
        let selected_date = *self.selected_date(app).value(app);
        let autovalidate_mode = *self.autovalidate_mode(app).value(app);

        // There's no M3 spec for a landscape layout input (not calendar) date picker. To
        // ensure that the date displayed in the input date picker's header fits in landscape
        // mode, we override the M3 default here.
        let mut headline_style: Option<TextStyle>;
        if use_material3 {
            headline_style = date_picker_theme
                .header_headline_style
                .clone()
                .or_else(|| defaults.header_headline_style(app));
            if matches!(
                entry_mode,
                DatePickerEntryMode::Input | DatePickerEntryMode::InputOnly
            ) && orientation == Orientation::Landscape
            {
                headline_style = text_theme.headline_small.clone();
            }
        } else {
            headline_style = if is_landscape_orientation {
                text_theme.headline_small.clone()
            } else {
                text_theme.headline_medium.clone()
            };
        }
        let header_foreground_color = date_picker_theme
            .header_foreground_color
            .or_else(|| defaults.header_foreground_color(app));
        headline_style = headline_style.map(|style| match header_foreground_color {
            Some(color) => style.copy_with().color(color),
            None => style,
        });

        let cancel_button_style = date_picker_theme
            .cancel_button_style
            .clone()
            .or_else(|| defaults.cancel_button_style(app));
        let confirm_button_style = date_picker_theme
            .confirm_button_style
            .clone()
            .or_else(|| defaults.confirm_button_style(app));
        let mut cancel_button = TextButton::new(
            Some(Listener::new(move |app| self.handle_cancel(app))),
            Some(Text::new(widget.cancel_text.clone().unwrap_or_else(|| {
                if use_material3 {
                    localizations.cancel_button_label()
                } else {
                    localizations.cancel_button_label().to_uppercase()
                }
            }))),
        );
        if let Some(style) = cancel_button_style {
            cancel_button = cancel_button.style(style);
        }
        let mut confirm_button = TextButton::new(
            Some(Listener::new(move |app| self.handle_ok(app))),
            Some(Text::new(
                widget
                    .confirm_text
                    .clone()
                    .unwrap_or_else(|| localizations.ok_button_label()),
            )),
        );
        if let Some(style) = confirm_button_style {
            confirm_button = confirm_button.style(style);
        }
        let actions = ConstrainedBox::new(BoxConstraints {
            min_height: 52.0,
            ..BoxConstraints::new()
        })
        .child(MediaQuery::with_clamped_text_scaling(
            None,
            0.0,
            if is_landscape_orientation {
                1.6
            } else {
                MAX_TEXT_SCALE_FACTOR
            },
            Padding::new(EdgeInsets::symmetric(0.0, 8.0).into()).child(
                Align::new()
                    .alignment(AlignmentGeometry::from(AlignmentDirectional::CENTER_END))
                    .child(OverflowBar::new().spacing(8.0).children(vec![
                        cancel_button.into_widget(),
                        confirm_button.into_widget(),
                    ])),
            ),
        ))
        .into_widget();

        let calendar_picker_key = app.get(self).calendar_picker_key.clone();
        let form_key = app.get(self).form_key.clone();
        let make_calendar_date_picker = {
            let widget = widget.clone();
            move || -> WidgetRef {
                let mut picker = CalendarDatePicker::new(
                    selected_date,
                    widget.first_date,
                    widget.last_date,
                    Rc::new(move |app, date| self.handle_date_changed(app, date)),
                )
                .calendar_delegate(Rc::clone(&widget.calendar_delegate))
                .key(calendar_picker_key.clone().into())
                .current_date(widget.current_date)
                .initial_calendar_mode(widget.initial_calendar_mode);
                if let Some(predicate) = widget.selectable_day_predicate.clone() {
                    picker = picker.selectable_day_predicate(predicate);
                }
                if let Some(day_builder) = widget.day_builder.clone() {
                    picker = picker.day_builder(day_builder);
                }
                if let Some(weekday_builder) = widget.weekday_builder.clone() {
                    picker = picker.weekday_builder(weekday_builder);
                }
                picker.into_widget()
            }
        };
        let make_input_date_picker = {
            let widget = widget.clone();
            move || -> WidgetRef {
                let mut field = InputDatePickerFormField::new(widget.first_date, widget.last_date)
                    .calendar_delegate(Rc::clone(&widget.calendar_delegate))
                    .on_date_submitted(Rc::new(move |app, date| {
                        self.handle_date_changed(app, date);
                    }))
                    .on_date_saved(Rc::new(move |app, date| {
                        self.handle_date_changed(app, date);
                    }))
                    .autofocus(true);
                if let Some(initial_date) = selected_date {
                    field = field.initial_date(initial_date);
                }
                if let Some(predicate) = widget.selectable_day_predicate.clone() {
                    field = field.selectable_day_predicate(predicate);
                }
                if let Some(text) = widget.error_format_text.clone() {
                    field = field.error_format_text(text);
                }
                if let Some(text) = widget.error_invalid_text.clone() {
                    field = field.error_invalid_text(text);
                }
                if let Some(text) = widget.field_hint_text.clone() {
                    field = field.field_hint_text(text);
                }
                if let Some(text) = widget.field_label_text.clone() {
                    field = field.field_label_text(text);
                }
                if let Some(keyboard_type) = widget.keyboard_type {
                    field = field.keyboard_type(keyboard_type);
                }
                Form::new(
                    SizedBox::new()
                        .height(if orientation == Orientation::Portrait {
                            INPUT_FORM_PORTRAIT_HEIGHT
                        } else {
                            INPUT_FORM_LANDSCAPE_HEIGHT
                        })
                        .child(
                            Padding::new(EdgeInsets::symmetric(0.0, 24.0).into()).child(
                                Shortcuts::new(
                                    DatePickerDialogState::form_shortcut_map(),
                                    Column::new()
                                        .main_axis_alignment(MainAxisAlignment::Center)
                                        .children([Flexible::new(
                                            MediaQuery::with_clamped_text_scaling(
                                                None, 0.0, 2.0, field,
                                            ),
                                        )
                                        .into_widget()]),
                                ),
                            ),
                        ),
                )
                .key(form_key.clone().into())
                .autovalidate_mode(autovalidate_mode)
                .into_widget()
            }
        };

        let (picker, entry_mode_button): (WidgetRef, Option<WidgetRef>) = match entry_mode {
            DatePickerEntryMode::Calendar => {
                let mut button = IconButton::new(
                    Some(Listener::new(move |app| self.handle_entry_mode_toggle(app))),
                    widget
                        .switch_to_input_entry_mode_icon
                        .clone()
                        .unwrap_or_else(|| {
                            Icon::new(Some(if use_material3 {
                                Icons::edit_outlined()
                            } else {
                                Icons::edit()
                            }))
                            .into_widget()
                        }),
                )
                .tooltip(localizations.input_date_mode_button_label());
                if let Some(color) = header_foreground_color {
                    button = button.color(color);
                }
                (make_calendar_date_picker(), Some(button.into_widget()))
            }
            DatePickerEntryMode::CalendarOnly => (make_calendar_date_picker(), None),
            DatePickerEntryMode::Input => {
                let mut button = IconButton::new(
                    Some(Listener::new(move |app| self.handle_entry_mode_toggle(app))),
                    widget
                        .switch_to_calendar_entry_mode_icon
                        .clone()
                        .unwrap_or_else(|| Icon::new(Some(Icons::calendar_today())).into_widget()),
                )
                .tooltip(localizations.calendar_mode_button_label());
                if let Some(color) = header_foreground_color {
                    button = button.color(color);
                }
                (make_input_date_picker(), Some(button.into_widget()))
            }
            DatePickerEntryMode::InputOnly => (make_input_date_picker(), None),
        };

        let header = DatePickerHeader {
            help_text: widget.help_text.clone().unwrap_or_else(|| {
                if use_material3 {
                    localizations.date_picker_help_text()
                } else {
                    localizations.date_picker_help_text().to_uppercase()
                }
            }),
            title_text: match selected_date {
                None => String::new(),
                Some(date) => widget
                    .calendar_delegate
                    .format_medium_date(&date, &*localizations),
            },
            title_semantics_label: None,
            title_style: headline_style,
            orientation,
            is_short: orientation == Orientation::Landscape,
            entry_mode_button,
        }
        .into_widget();

        // Constrain the textScaleFactor to the largest supported value to prevent layout
        // issues.
        let text_scale_factor = MediaQuery::text_scaler_of(app, context)
            .clamp(0.0, MAX_TEXT_SCALE_FACTOR)
            .scale(FONT_SIZE_TO_SCALE)
            / FONT_SIZE_TO_SCALE;
        let dialog_size = self.dialog_size(app, context) * text_scale_factor;
        let dialog_theme = theme.dialog_theme.clone();
        let divider_color = date_picker_theme.divider_color;

        let mut dialog = Dialog::new()
            .elevation(if use_material3 {
                date_picker_theme
                    .elevation
                    .or_else(|| defaults.elevation(app))
                    .expect("the defaults give an elevation")
            } else {
                date_picker_theme
                    .elevation
                    .or(dialog_theme.elevation)
                    .unwrap_or(24.0)
            })
            .inset_padding(widget.inset_padding)
            .clip_behavior(Clip::AntiAlias);
        if let Some(color) = date_picker_theme
            .background_color
            .or_else(|| defaults.background_color(app))
        {
            dialog = dialog.background_color(color);
        }
        if let Some(color) = date_picker_theme
            .shadow_color
            .or_else(|| defaults.shadow_color(app))
        {
            dialog = dialog.shadow_color(color);
        }
        if let Some(color) = date_picker_theme
            .surface_tint_color
            .or_else(|| defaults.surface_tint_color(app))
        {
            dialog = dialog.surface_tint_color(color);
        }
        let shape = if use_material3 {
            date_picker_theme
                .shape
                .clone()
                .or_else(|| defaults.shape(app))
        } else {
            date_picker_theme
                .shape
                .clone()
                .or_else(|| dialog_theme.shape.clone())
                .or_else(|| defaults.shape(app))
        };
        if let Some(shape) = shape {
            dialog = dialog.shape(shape);
        }

        dialog
            .child(
                AnimatedContainer::new(DIALOG_SIZE_ANIMATION_DURATION)
                    .width(dialog_size.width())
                    .height(dialog_size.height())
                    .curve(Curves::ease_in())
                    .child(MediaQuery::with_clamped_text_scaling(
                        None,
                        0.0,
                        MAX_TEXT_SCALE_FACTOR,
                        LayoutBuilder::new(move |_app, _context, constraints| {
                            let portrait_dialog_size = if use_material3 {
                                INPUT_PORTRAIT_DIALOG_SIZE_M3
                            } else {
                                INPUT_PORTRAIT_DIALOG_SIZE_M2
                            };
                            // Make sure the portrait dialog can fit the contents comfortably
                            // when resized from the landscape dialog.
                            let is_fully_portrait = constraints.constraints().max_height
                                >= dialog_size.height().min(portrait_dialog_size.height());
                            match orientation {
                                Orientation::Portrait => {
                                    let is_input_mode = matches!(
                                        entry_mode,
                                        DatePickerEntryMode::InputOnly | DatePickerEntryMode::Input
                                    );
                                    // When the portrait dialog does not fit vertically, hide
                                    // the header when the entry mode is input, or hide the
                                    // picker when the entry mode is not input.
                                    let show_header = is_fully_portrait || !is_input_mode;
                                    let show_picker = is_fully_portrait || is_input_mode;
                                    let mut children: Vec<WidgetRef> = Vec::new();
                                    if show_header {
                                        children.push(header.clone());
                                    }
                                    if use_material3 {
                                        let mut divider = Divider::new().height(0.0);
                                        if let Some(color) = divider_color {
                                            divider = divider.color(color);
                                        }
                                        children.push(divider.into_widget());
                                    }
                                    if show_picker {
                                        children.push(Expanded::new(picker.clone()).into_widget());
                                        children.push(actions.clone());
                                    }
                                    Column::new()
                                        .main_axis_size(MainAxisSize::Min)
                                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                        .children(children)
                                        .into_widget()
                                }
                                Orientation::Landscape => {
                                    let mut children = vec![header.clone()];
                                    if use_material3 {
                                        let mut divider = VerticalDivider::new().width(0.0);
                                        if let Some(color) = divider_color {
                                            divider = divider.color(color);
                                        }
                                        children.push(divider.into_widget());
                                    }
                                    children.push(
                                        Flexible::new(
                                            Column::new()
                                                .main_axis_size(MainAxisSize::Min)
                                                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                                .children([
                                                    Expanded::new(picker.clone()).into_widget(),
                                                    actions.clone(),
                                                ]),
                                        )
                                        .into_widget(),
                                    );
                                    Row::new()
                                        .main_axis_size(MainAxisSize::Min)
                                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                        .children(children)
                                        .into_widget()
                                }
                            }
                        })
                        .into_widget(),
                    )),
            )
            .into_widget()
    }
}

impl std::fmt::Debug for DatePickerDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatePickerDialog")
            .field("initial_date", &self.initial_date)
            .field("first_date", &self.first_date)
            .field("last_date", &self.last_date)
            .field("current_date", &self.current_date)
            .field("initial_entry_mode", &self.initial_entry_mode)
            .field("initial_calendar_mode", &self.initial_calendar_mode)
            .field("restoration_id", &self.restoration_id)
            .finish_non_exhaustive()
    }
}

const DATE_PICKER_HEADER_LANDSCAPE_WIDTH: f64 = 152.0;
const DATE_PICKER_HEADER_PORTRAIT_HEIGHT: f64 = 120.0;
const HEADER_PADDING_LANDSCAPE: f64 = 16.0;

/// Dart's `_DatePickerHeader`.
#[derive(Clone, Debug)]
struct DatePickerHeader {
    help_text: String,
    title_text: String,
    title_semantics_label: Option<String>,
    title_style: Option<TextStyle>,
    orientation: Orientation,
    is_short: bool,
    entry_mode_button: Option<WidgetRef>,
}

impl StatelessWidget for DatePickerHeader {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let background_color = date_picker_theme
            .header_background_color
            .or_else(|| defaults.header_background_color(app));
        let foreground_color = date_picker_theme
            .header_foreground_color
            .or_else(|| defaults.header_foreground_color(app));
        let help_style = date_picker_theme
            .header_help_style
            .clone()
            .or_else(|| defaults.header_help_style(app))
            .map(|style| match foreground_color {
                Some(color) => style.copy_with().color(color),
                None => style,
            });
        let text_scaler = MediaQuery::text_scaler_of(app, context);
        let current_scale = text_scaler.scale(FONT_SIZE_TO_SCALE) / FONT_SIZE_TO_SCALE;
        let max_header_text_scale_factor = current_scale.min(if self.entry_mode_button.is_some() {
            MAX_HEADER_WITH_ENTRY_TEXT_SCALE_FACTOR
        } else {
            MAX_HEADER_TEXT_SCALE_FACTOR
        });
        let text_scale_factor = text_scaler
            .clamp(0.0, max_header_text_scale_factor)
            .scale(FONT_SIZE_TO_SCALE)
            / FONT_SIZE_TO_SCALE;
        let scaled_font_size = text_scaler.scale(
            self.title_style
                .as_ref()
                .and_then(|style| style.font_size)
                .unwrap_or(32.0),
        );
        let header_scale_factor = if text_scale_factor > 1.0 {
            text_scale_factor
        } else {
            1.0
        };

        let mut help = Text::new(self.help_text.clone())
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis)
            .text_scaler(text_scaler.clamp(
                0.0,
                text_scale_factor.min(if self.orientation == Orientation::Portrait {
                    MAX_HELP_PORTRAIT_TEXT_SCALE_FACTOR
                } else {
                    MAX_HELP_LANDSCAPE_TEXT_SCALE_FACTOR
                }),
            ));
        if let Some(help_style) = help_style {
            help = help.style(help_style);
        }
        let mut title = Text::new(self.title_text.clone())
            .semantics_label(
                self.title_semantics_label
                    .clone()
                    .unwrap_or_else(|| self.title_text.clone()),
            )
            .max_lines(if self.orientation == Orientation::Portrait {
                if scaled_font_size > 70.0 { 2 } else { 1 }
            } else if scaled_font_size > 40.0 {
                3
            } else {
                2
            })
            .overflow(TextOverflow::Ellipsis)
            .text_scaler(text_scaler.clamp(0.0, text_scale_factor));
        if let Some(title_style) = self.title_style.clone() {
            title = title.style(title_style);
        }

        let font_scale_adjusted_header_height = if header_scale_factor > 1.3 {
            header_scale_factor - 0.2
        } else {
            1.0
        };

        let mut material = Material::new();
        if let Some(background_color) = background_color {
            material = material.color(background_color);
        }
        match self.orientation {
            Orientation::Portrait => {
                let mut title_row = vec![Expanded::new(title).into_widget()];
                if let Some(entry_mode_button) = &self.entry_mode_button {
                    title_row.push(
                        Semantics::new()
                            .container(true)
                            .child(entry_mode_button.clone())
                            .into_widget(),
                    );
                }
                Semantics::new()
                    .container(true)
                    .child(
                        SizedBox::new()
                            .height(
                                DATE_PICKER_HEADER_PORTRAIT_HEIGHT
                                    * font_scale_adjusted_header_height,
                            )
                            .child(
                                material.child(
                                    Padding::new(
                                        EdgeInsetsDirectional::only(24.0, 0.0, 12.0, 12.0).into(),
                                    )
                                    .child(
                                        Column::new()
                                            .cross_axis_alignment(CrossAxisAlignment::Start)
                                            .children([
                                                SizedBox::new().height(16.0).into_widget(),
                                                help.into_widget(),
                                                Flexible::new(SizedBox::new().height(38.0))
                                                    .into_widget(),
                                                Row::new().children(title_row).into_widget(),
                                            ]),
                                    ),
                                ),
                            ),
                    )
                    .into_widget()
            }
            Orientation::Landscape => {
                let mut children = vec![
                    SizedBox::new().height(16.0).into_widget(),
                    Padding::new(EdgeInsets::symmetric(0.0, HEADER_PADDING_LANDSCAPE).into())
                        .child(help)
                        .into_widget(),
                    SizedBox::new()
                        .height(if self.is_short { 16.0 } else { 56.0 })
                        .into_widget(),
                    Expanded::new(
                        Padding::new(EdgeInsets::symmetric(0.0, HEADER_PADDING_LANDSCAPE).into())
                            .child(title),
                    )
                    .into_widget(),
                ];
                if let Some(entry_mode_button) = &self.entry_mode_button {
                    children.push(
                        Padding::new(if theme.use_material3 {
                            EdgeInsetsDirectional::only(8.0, 0.0, 4.0, 6.0).into()
                        } else {
                            EdgeInsets::symmetric(0.0, 4.0).into()
                        })
                        .child(
                            Semantics::new()
                                .container(true)
                                .child(entry_mode_button.clone()),
                        )
                        .into_widget(),
                    );
                }
                Semantics::new()
                    .container(true)
                    .child(
                        SizedBox::new()
                            .width(DATE_PICKER_HEADER_LANDSCAPE_WIDTH)
                            .child(
                                material.child(
                                    Column::new()
                                        .cross_axis_alignment(CrossAxisAlignment::Start)
                                        .children(children),
                                ),
                            ),
                    )
                    .into_widget()
            }
        }
    }
}

/// Signature for predicates for enabling a day in a date range picker.
///
/// It receives the day, and the start and end of the selected range (either may be null).
/// Days for which it returns false are not selectable.
pub type SelectableDayForRangePredicate =
    Rc<dyn Fn(DateTime, Option<DateTime>, Option<DateTime>) -> bool>;

/// Shows a full screen modal dialog containing a Material Design date range picker.
///
/// The returned future resolves when the user closes the dialog. If the user cancels the
/// dialog, it resolves with null; otherwise it resolves to the selected [`DateTimeRange`]
/// (inside the [`RouteResult`]).
///
/// The `initial_date_range` is the range the picker opens with. The `first_date` and `last_date`
/// are the allowable range of dates. The `current_date` is highlighted in the day grid; with
/// none, the delegate's `now()` is used.
///
/// The `initial_entry_mode` chooses between the calendar and the text input.
///
/// The `selectable_day_predicate` customizes the days enabled for selection.
///
/// Dart's `anchorPoint` waits for `show_dialog` to take one.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_date_range_picker(
    app: &mut App,
    context: BuildContext,
    initial_date_range: Option<DateTimeRange>,
    first_date: DateTime,
    last_date: DateTime,
    current_date: Option<DateTime>,
    initial_entry_mode: DatePickerEntryMode,
    help_text: Option<String>,
    cancel_text: Option<String>,
    confirm_text: Option<String>,
    save_text: Option<String>,
    error_format_text: Option<String>,
    error_invalid_text: Option<String>,
    error_invalid_range_text: Option<String>,
    field_start_hint_text: Option<String>,
    field_end_hint_text: Option<String>,
    field_start_label_text: Option<String>,
    field_end_label_text: Option<String>,
    locale: Option<Locale>,
    barrier_dismissible: bool,
    barrier_color: Option<Color>,
    barrier_label: Option<String>,
    use_root_navigator: bool,
    route_settings: Option<RouteSettingsRef>,
    text_direction: Option<TextDirection>,
    builder: Option<TransitionBuilder>,
    keyboard_type: TextInputType,
    switch_to_input_entry_mode_icon: Option<Icon>,
    switch_to_calendar_entry_mode_icon: Option<Icon>,
    selectable_day_predicate: Option<SelectableDayForRangePredicate>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
) -> CompleterFuture<RouteResult> {
    let initial_date_range = initial_date_range.map(|range| calendar_delegate.dates_only(&range));
    let first_date = calendar_delegate.date_only(&first_date);
    let last_date = calendar_delegate.date_only(&last_date);
    assert!(
        !last_date.is_before(&first_date),
        "lastDate {last_date} must be on or after firstDate {first_date}."
    );
    if let Some(range) = &initial_date_range {
        assert!(
            !range.start.is_before(&first_date),
            "initialDateRange's start date must be on or after firstDate {first_date}."
        );
        assert!(
            !range.end.is_before(&first_date),
            "initialDateRange's end date must be on or after firstDate {first_date}."
        );
        assert!(
            !range.start.is_after(&last_date),
            "initialDateRange's start date must be on or before lastDate {last_date}."
        );
        assert!(
            !range.end.is_after(&last_date),
            "initialDateRange's end date must be on or before lastDate {last_date}."
        );
        if let Some(predicate) = &selectable_day_predicate {
            assert!(
                predicate(range.start, Some(range.start), Some(range.end)),
                "initialDateRange's start date must be selectable."
            );
            assert!(
                predicate(range.end, Some(range.start), Some(range.end)),
                "initialDateRange's end date must be selectable."
            );
        }
    }
    let current_date =
        calendar_delegate.date_only(&current_date.unwrap_or_else(|| calendar_delegate.now()));
    debug_assert!(crate::debug::debug_check_has_material_localizations(
        app, context
    ));

    let mut dialog = DateRangePickerDialog::new(first_date, last_date)
        .calendar_delegate(Rc::clone(&calendar_delegate))
        .initial_entry_mode(initial_entry_mode)
        .keyboard_type(keyboard_type);
    dialog.initial_date_range = initial_date_range;
    dialog.current_date_value = Some(current_date);
    dialog.selectable_day_predicate = selectable_day_predicate;
    dialog.help_text = help_text;
    dialog.cancel_text = cancel_text;
    dialog.confirm_text = confirm_text;
    dialog.save_text = save_text;
    dialog.error_format_text = error_format_text;
    dialog.error_invalid_text = error_invalid_text;
    dialog.error_invalid_range_text = error_invalid_range_text;
    dialog.field_start_hint_text = field_start_hint_text;
    dialog.field_end_hint_text = field_end_hint_text;
    dialog.field_start_label_text = field_start_label_text;
    dialog.field_end_label_text = field_end_label_text;
    dialog.switch_to_input_entry_mode_icon =
        switch_to_input_entry_mode_icon.map(IntoWidget::into_widget);
    dialog.switch_to_calendar_entry_mode_icon =
        switch_to_calendar_entry_mode_icon.map(IntoWidget::into_widget);
    let mut dialog = dialog.into_widget();

    if let Some(text_direction) = text_direction {
        dialog = Directionality::new(text_direction, dialog).into_widget();
    }

    if let Some(locale) = locale {
        dialog = Localizations::r#override(app, context)
            .locale(locale)
            .child(dialog)
            .into_widget();
    }

    crate::dialog::show_dialog(
        app,
        context,
        Rc::new(move |app, context| match &builder {
            None => dialog.clone(),
            Some(builder) => builder(app, context, Some(&dialog)),
        }),
        barrier_dismissible,
        barrier_color,
        barrier_label,
        false,
        use_root_navigator,
        route_settings,
        None,
        false,
        None,
        None,
    )
}

/// Dart's `_formatRangeStartDate`.
fn format_range_start_date(
    localizations: &dyn MaterialLocalizations,
    calendar_delegate: &dyn CalendarDelegate,
    start_date: Option<&DateTime>,
    end_date: Option<&DateTime>,
) -> String {
    match start_date {
        None => localizations.date_range_start_label(),
        Some(start) if end_date.is_none_or(|end| start.year() == end.year()) => {
            calendar_delegate.format_short_month_day(start, localizations)
        }
        Some(start) => calendar_delegate.format_short_date(start, localizations),
    }
}

/// Dart's `_formatRangeEndDate`.
fn format_range_end_date(
    localizations: &dyn MaterialLocalizations,
    calendar_delegate: &dyn CalendarDelegate,
    start_date: Option<&DateTime>,
    end_date: Option<&DateTime>,
    current_date: &DateTime,
) -> String {
    match end_date {
        None => localizations.date_range_end_label(),
        Some(end)
            if start_date.is_some_and(|start| {
                start.year() == end.year() && start.year() == current_date.year()
            }) =>
        {
            calendar_delegate.format_short_month_day(end, localizations)
        }
        Some(end) => calendar_delegate.format_short_date(end, localizations),
    }
}

/// A Material-style date range picker dialog.
///
/// It is used by [`show_date_range_picker`], which should usually be called rather than
/// creating a [`DateRangePickerDialog`] directly.
#[derive(Clone)]
pub struct DateRangePickerDialog {
    pub key: Option<waft_widgets::KeyRef>,

    /// The date range that the date range picker starts with when it opens.
    ///
    /// If an initial date range is provided, [`DateTimeRange::start`] and [`DateTimeRange::end`]
    /// must both fall within the [`first_date`](Self::first_date) and
    /// [`last_date`](Self::last_date) range.
    pub initial_date_range: Option<DateTimeRange>,

    /// The earliest allowable date on the date range.
    pub first_date: DateTime,

    /// The latest allowable date on the date range.
    pub last_date: DateTime,

    /// The [`current_date`](Self::current_date) value given, if any; the getter truncates it.
    pub current_date_value: Option<DateTime>,

    /// The initial mode of date entry method for the date range picker dialog.
    pub initial_entry_mode: DatePickerEntryMode,

    /// The label on the cancel button for the text input mode.
    pub cancel_text: Option<String>,

    /// The label on the "OK" button for the text input mode.
    pub confirm_text: Option<String>,

    /// The label on the save button for the fullscreen calendar mode.
    pub save_text: Option<String>,

    /// The label displayed at the top of the dialog.
    pub help_text: Option<String>,

    /// Error text used to indicate the entered date range is invalid.
    pub error_invalid_range_text: Option<String>,

    /// Error text used to indicate the entered date format is invalid.
    pub error_format_text: Option<String>,

    /// Error text used to indicate that an entered date is outside the allowed range.
    pub error_invalid_text: Option<String>,

    /// Hint text shown when the start date field is empty.
    pub field_start_hint_text: Option<String>,

    /// Hint text shown when the end date field is empty.
    pub field_end_hint_text: Option<String>,

    /// Label used for the start date input field in the input mode.
    pub field_start_label_text: Option<String>,

    /// Label used for the end date input field in the input mode.
    pub field_end_label_text: Option<String>,

    /// The keyboard type of the date fields.
    pub keyboard_type: TextInputType,

    /// Restoration ID to save and restore the state of the [`DateRangePickerDialog`].
    pub restoration_id: Option<String>,

    /// An optional icon that, when pressed, switches from the calendar to the text input.
    pub switch_to_input_entry_mode_icon: Option<WidgetRef>,

    /// An optional icon that, when pressed, switches from the text input to the calendar.
    pub switch_to_calendar_entry_mode_icon: Option<WidgetRef>,

    /// Function to provide full control over which [`DateTime`] can be selected.
    pub selectable_day_predicate: Option<SelectableDayForRangePredicate>,

    /// The calendar the dialog works in.
    pub calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl DateRangePickerDialog {
    /// Creates a [`DateRangePickerDialog`]. Dart's optional named arguments are the fluent
    /// setters.
    pub fn new(first_date: DateTime, last_date: DateTime) -> DateRangePickerDialog {
        DateRangePickerDialog {
            key: None,
            initial_date_range: None,
            first_date,
            last_date,
            current_date_value: None,
            initial_entry_mode: DatePickerEntryMode::Calendar,
            cancel_text: None,
            confirm_text: None,
            save_text: None,
            help_text: None,
            error_invalid_range_text: None,
            error_format_text: None,
            error_invalid_text: None,
            field_start_hint_text: None,
            field_end_hint_text: None,
            field_start_label_text: None,
            field_end_label_text: None,
            keyboard_type: TextInputType::DATETIME,
            restoration_id: None,
            switch_to_input_entry_mode_icon: None,
            switch_to_calendar_entry_mode_icon: None,
            selectable_day_predicate: None,
            calendar_delegate: Rc::new(GregorianCalendarDelegate),
        }
    }

    /// The [`DateTime`] representing today. It will be highlighted in the day grid.
    pub fn current_date(&self) -> DateTime {
        self.calendar_delegate.date_only(
            &self
                .current_date_value
                .unwrap_or_else(|| self.calendar_delegate.now()),
        )
    }

    /// Dart `DateRangePickerDialog(key:)`.
    pub fn key(mut self, key: waft_widgets::KeyRef) -> DateRangePickerDialog {
        self.key = Some(key);
        self
    }

    /// Dart `DateRangePickerDialog(initialDateRange:)`.
    pub fn initial_date_range(
        mut self,
        initial_date_range: DateTimeRange,
    ) -> DateRangePickerDialog {
        self.initial_date_range = Some(initial_date_range);
        self
    }

    /// Dart `DateRangePickerDialog(currentDate:)`.
    pub fn with_current_date(mut self, current_date: DateTime) -> DateRangePickerDialog {
        self.current_date_value = Some(current_date);
        self
    }

    /// Dart `DateRangePickerDialog(initialEntryMode:)`.
    pub fn initial_entry_mode(
        mut self,
        initial_entry_mode: DatePickerEntryMode,
    ) -> DateRangePickerDialog {
        self.initial_entry_mode = initial_entry_mode;
        self
    }

    /// Dart `DateRangePickerDialog(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> DateRangePickerDialog {
        self.keyboard_type = keyboard_type;
        self
    }

    /// Dart `DateRangePickerDialog(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> DateRangePickerDialog {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `DateRangePickerDialog(selectableDayPredicate:)`.
    pub fn selectable_day_predicate(
        mut self,
        selectable_day_predicate: SelectableDayForRangePredicate,
    ) -> DateRangePickerDialog {
        self.selectable_day_predicate = Some(selectable_day_predicate);
        self
    }

    /// Dart `DateRangePickerDialog(calendarDelegate:)`.
    pub fn calendar_delegate(
        mut self,
        calendar_delegate: Rc<dyn CalendarDelegate>,
    ) -> DateRangePickerDialog {
        self.calendar_delegate = calendar_delegate;
        self
    }
}

impl fmt::Debug for DateRangePickerDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DateRangePickerDialog")
            .field("initial_date_range", &self.initial_date_range)
            .field("first_date", &self.first_date)
            .field("last_date", &self.last_date)
            .field("initial_entry_mode", &self.initial_entry_mode)
            .field("restoration_id", &self.restoration_id)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for DateRangePickerDialog {
    type State = DateRangePickerDialogState;

    fn key(&self) -> Option<&waft_widgets::KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DateRangePickerDialogState {
        DateRangePickerDialogState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            entry_mode: None,
            selected_start: None,
            selected_end: None,
            auto_validate: None,
            calendar_picker_key: waft_widgets::GlobalKey::new(),
            input_picker_key: waft_widgets::GlobalKey::new(),
        }
    }
}

/// Dart's `_DateRangePickerDialogState`.
pub struct DateRangePickerDialogState {
    state: StateData<DateRangePickerDialog>,
    restoration: RestorationMixinData,
    entry_mode: Option<Ref<RestorableEnum<DatePickerEntryMode>>>,
    selected_start: Option<Ref<RestorableDateTimeN>>,
    selected_end: Option<Ref<RestorableDateTimeN>>,
    auto_validate: Option<Ref<RestorableBool>>,
    calendar_picker_key: waft_widgets::GlobalKey,
    input_picker_key: waft_widgets::GlobalKey,
}

impl DateRangePickerDialogState {
    fn entry_mode(self: Handle<Self>, app: &App) -> Handle<RestorableEnum<DatePickerEntryMode>> {
        app.get(self)
            .entry_mode
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn selected_start(self: Handle<Self>, app: &App) -> Handle<RestorableDateTimeN> {
        app.get(self)
            .selected_start
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn selected_end(self: Handle<Self>, app: &App) -> Handle<RestorableDateTimeN> {
        app.get(self)
            .selected_end
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn auto_validate(self: Handle<Self>, app: &App) -> Handle<RestorableBool> {
        app.get(self)
            .auto_validate
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn has_selected_date_range(self: Handle<Self>, app: &App) -> bool {
        self.selected_start(app).value(app).is_some() && self.selected_end(app).value(app).is_some()
    }

    fn handle_ok(self: Handle<Self>, app: &mut App) {
        let entry_mode = *self.entry_mode(app).value(app);
        if entry_mode == DatePickerEntryMode::Input || entry_mode == DatePickerEntryMode::InputOnly
        {
            let input_picker_key = app.get(self).input_picker_key.clone();
            let picker = input_picker_key
                .current_state::<InputDateRangePickerState>(app)
                .expect("the input picker is mounted");
            if !picker.validate(app) {
                self.set_state(app, |_state| {});
                self.auto_validate(app).set_value(app, true);
                return;
            }
        }
        let selected_range = match (
            *self.selected_start(app).value(app),
            *self.selected_end(app).value(app),
        ) {
            (Some(start), Some(end)) => {
                Some(Rc::new(DateTimeRange::new(start, end)) as Rc<dyn std::any::Any>)
            }
            _ => None,
        };
        let context = self.context(app);
        Navigator::pop(app, context, selected_range);
    }

    fn handle_cancel(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        Navigator::pop(app, context, None);
    }

    fn is_day_selectable(self: Handle<Self>, app: &App, day: DateTime) -> bool {
        let widget = self.widget(app);
        if day.is_before(&widget.first_date) || day.is_after(&widget.last_date) {
            return false;
        }
        match &widget.selectable_day_predicate {
            None => true,
            Some(predicate) => predicate(
                day,
                *self.selected_start(app).value(app),
                *self.selected_end(app).value(app),
            ),
        }
    }

    fn handle_entry_mode_toggle(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
        match *self.entry_mode(app).value(app) {
            DatePickerEntryMode::Calendar => {
                self.auto_validate(app).set_value(app, false);
                self.entry_mode(app)
                    .set_value(app, DatePickerEntryMode::Input);
            }
            DatePickerEntryMode::Input => {
                // Validate the range dates
                let start = *self.selected_start(app).value(app);
                let end = *self.selected_end(app).value(app);
                if let (Some(start), Some(end)) = (start, end)
                    && start.is_after(&end)
                {
                    self.selected_end(app).set_value(app, None);
                }
                if let Some(start) = *self.selected_start(app).value(app)
                    && !self.is_day_selectable(app, start)
                {
                    self.selected_start(app).set_value(app, None);
                    // With no valid start date, having an end date makes no sense for the UI.
                    self.selected_end(app).set_value(app, None);
                } else if let Some(end) = *self.selected_end(app).value(app)
                    && !self.is_day_selectable(app, end)
                {
                    self.selected_end(app).set_value(app, None);
                }
                self.entry_mode(app)
                    .set_value(app, DatePickerEntryMode::Calendar);
            }
            DatePickerEntryMode::CalendarOnly | DatePickerEntryMode::InputOnly => {
                debug_assert!(
                    false,
                    "Can not change entry mode from {:?}",
                    *self.entry_mode(app).value(app)
                );
            }
        }
    }

    fn handle_start_date_changed(self: Handle<Self>, app: &mut App, date: Option<DateTime>) {
        self.set_state(app, |_state| {});
        self.selected_start(app).set_value(app, date);
    }

    fn handle_end_date_changed(self: Handle<Self>, app: &mut App, date: Option<DateTime>) {
        self.set_state(app, |_state| {});
        self.selected_end(app).set_value(app, date);
    }
}

impl RestorationMixin for DateRangePickerDialogState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<waft_widgets::RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let (entry_mode, selected_start, selected_end, auto_validate) = (
            self.entry_mode(app),
            self.selected_start(app),
            self.selected_end(app),
            self.auto_validate(app),
        );
        self.register_for_restoration(app, entry_mode.as_property(), "entry_mode");
        self.register_for_restoration(app, selected_start.as_property(), "selected_start");
        self.register_for_restoration(app, selected_end.as_property(), "selected_end");
        self.register_for_restoration(app, auto_validate.as_property(), "autovalidate");
    }
}

impl State for DateRangePickerDialogState {
    type Widget = DateRangePickerDialog;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let entry_mode = RestorableEnum::new(app, widget.initial_entry_mode, ENTRY_MODES);
        let selected_start =
            RestorableDateTimeN::new(app, widget.initial_date_range.map(|range| range.start));
        let selected_end =
            RestorableDateTimeN::new(app, widget.initial_date_range.map(|range| range.end));
        let auto_validate = RestorableBool::new(app, false);
        let state = app.get_mut(self);
        state.entry_mode = Some(entry_mode);
        state.selected_start = Some(selected_start);
        state.selected_end = Some(selected_end);
        state.auto_validate = Some(auto_validate);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &DateRangePickerDialog) {
        self.did_update_restoration_id(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.entry_mode(app).dispose(app);
        self.selected_start(app).dispose(app);
        self.selected_end(app).dispose(app);
        self.auto_validate(app).dispose(app);
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let use_material3 = theme.use_material3;
        let orientation = MediaQuery::orientation_of(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let widget = self.widget(app).clone();
        let entry_mode = *self.entry_mode(app).value(app);
        let selected_start = *self.selected_start(app).value(app);
        let selected_end = *self.selected_end(app).value(app);
        let auto_validate = *self.auto_validate(app).value(app);
        let show_entry_mode_button = matches!(
            entry_mode,
            DatePickerEntryMode::Calendar | DatePickerEntryMode::Input
        );

        let contents: WidgetRef;
        let size: Size;
        let elevation: f64;
        let shadow_color: Option<Color>;
        let surface_tint_color: Option<Color>;
        let shape: Option<ShapeBorderRef>;
        let inset_padding: EdgeInsets;
        match entry_mode {
            DatePickerEntryMode::Calendar | DatePickerEntryMode::CalendarOnly => {
                let entry_mode_button = show_entry_mode_button.then(|| {
                    IconButton::new(
                        Some(Listener::new(move |app| self.handle_entry_mode_toggle(app))),
                        widget
                            .switch_to_input_entry_mode_icon
                            .clone()
                            .unwrap_or_else(|| {
                                Icon::new(Some(if use_material3 {
                                    Icons::edit_outlined()
                                } else {
                                    Icons::edit()
                                }))
                                .into_widget()
                            }),
                    )
                    .padding(EdgeInsets::ZERO.into())
                    .tooltip(localizations.input_date_mode_button_label())
                    .into_widget()
                });
                let calendar_picker_key = app.get(self).calendar_picker_key.clone();
                contents = CalendarRangePickerDialog {
                    key: Some(calendar_picker_key.into()),
                    calendar_delegate: Rc::clone(&widget.calendar_delegate),
                    selected_start_date: selected_start,
                    selected_end_date: selected_end,
                    first_date: widget.first_date,
                    last_date: widget.last_date,
                    selectable_day_predicate: widget.selectable_day_predicate.clone(),
                    current_date: widget.current_date(),
                    on_start_date_changed: Rc::new(move |app, date| {
                        self.handle_start_date_changed(app, date);
                    }),
                    on_end_date_changed: Rc::new(move |app, date| {
                        self.handle_end_date_changed(app, date);
                    }),
                    on_confirm: self
                        .has_selected_date_range(app)
                        .then(|| Listener::new(move |app| self.handle_ok(app))),
                    on_cancel: Listener::new(move |app| self.handle_cancel(app)),
                    entry_mode_button,
                    confirm_text: widget.save_text.clone().unwrap_or_else(|| {
                        if use_material3 {
                            localizations.save_button_label()
                        } else {
                            localizations.save_button_label().to_uppercase()
                        }
                    }),
                    help_text: widget.help_text.clone().unwrap_or_else(|| {
                        if use_material3 {
                            localizations.date_range_picker_help_text()
                        } else {
                            localizations.date_range_picker_help_text().to_uppercase()
                        }
                    }),
                }
                .into_widget();
                size = MediaQuery::size_of(app, context);
                inset_padding = EdgeInsets::ZERO;
                elevation = date_picker_theme
                    .range_picker_elevation
                    .or_else(|| defaults.range_picker_elevation(app))
                    .expect("the defaults give a range picker elevation");
                shadow_color = date_picker_theme
                    .range_picker_shadow_color
                    .or_else(|| defaults.range_picker_shadow_color(app));
                surface_tint_color = date_picker_theme
                    .range_picker_surface_tint_color
                    .or_else(|| defaults.range_picker_surface_tint_color(app));
                shape = date_picker_theme
                    .range_picker_shape
                    .clone()
                    .or_else(|| defaults.range_picker_shape(app));
            }
            DatePickerEntryMode::Input | DatePickerEntryMode::InputOnly => {
                let entry_mode_button = show_entry_mode_button.then(|| {
                    IconButton::new(
                        Some(Listener::new(move |app| self.handle_entry_mode_toggle(app))),
                        widget
                            .switch_to_calendar_entry_mode_icon
                            .clone()
                            .unwrap_or_else(|| {
                                Icon::new(Some(Icons::calendar_today())).into_widget()
                            }),
                    )
                    .padding(EdgeInsets::ZERO.into())
                    .tooltip(localizations.calendar_mode_button_label())
                    .into_widget()
                });
                let input_picker_key = app.get(self).input_picker_key.clone();
                let picker = SizedBox::new()
                    .height(if orientation == Orientation::Portrait {
                        INPUT_FORM_PORTRAIT_HEIGHT
                    } else {
                        INPUT_FORM_LANDSCAPE_HEIGHT
                    })
                    .child(
                        Padding::new(EdgeInsets::symmetric(0.0, 24.0).into()).child(
                            Column::new().children([
                                Spacer::new().into_widget(),
                                InputDateRangePicker {
                                    key: Some(input_picker_key.into()),
                                    calendar_delegate: Rc::clone(&widget.calendar_delegate),
                                    initial_start_date: selected_start,
                                    initial_end_date: selected_end,
                                    first_date: widget.first_date,
                                    last_date: widget.last_date,
                                    selectable_day_predicate: widget
                                        .selectable_day_predicate
                                        .clone(),
                                    on_start_date_changed: Some(Rc::new(move |app, date| {
                                        self.handle_start_date_changed(app, date);
                                    })),
                                    on_end_date_changed: Some(Rc::new(move |app, date| {
                                        self.handle_end_date_changed(app, date);
                                    })),
                                    autofocus: true,
                                    autovalidate: auto_validate,
                                    help_text: widget.help_text.clone(),
                                    error_invalid_range_text: widget
                                        .error_invalid_range_text
                                        .clone(),
                                    error_format_text: widget.error_format_text.clone(),
                                    error_invalid_text: widget.error_invalid_text.clone(),
                                    field_start_hint_text: widget.field_start_hint_text.clone(),
                                    field_end_hint_text: widget.field_end_hint_text.clone(),
                                    field_start_label_text: widget.field_start_label_text.clone(),
                                    field_end_label_text: widget.field_end_label_text.clone(),
                                    keyboard_type: widget.keyboard_type,
                                }
                                .into_widget(),
                                Spacer::new().into_widget(),
                            ]),
                        ),
                    )
                    .into_widget();
                contents = InputDateRangePickerDialog {
                    calendar_delegate: Rc::clone(&widget.calendar_delegate),
                    selected_start_date: selected_start,
                    selected_end_date: selected_end,
                    current_date: widget.current_date(),
                    picker,
                    on_confirm: Listener::new(move |app| self.handle_ok(app)),
                    on_cancel: Listener::new(move |app| self.handle_cancel(app)),
                    entry_mode_button,
                    confirm_text: widget
                        .confirm_text
                        .clone()
                        .unwrap_or_else(|| localizations.ok_button_label()),
                    cancel_text: widget.cancel_text.clone().unwrap_or_else(|| {
                        if use_material3 {
                            localizations.cancel_button_label()
                        } else {
                            localizations.cancel_button_label().to_uppercase()
                        }
                    }),
                    help_text: widget.help_text.clone().unwrap_or_else(|| {
                        if use_material3 {
                            localizations.date_range_picker_help_text()
                        } else {
                            localizations.date_range_picker_help_text().to_uppercase()
                        }
                    }),
                }
                .into_widget();
                let dialog_theme = theme.dialog_theme.clone();
                size = if orientation == Orientation::Portrait {
                    if use_material3 {
                        INPUT_PORTRAIT_DIALOG_SIZE_M3
                    } else {
                        INPUT_PORTRAIT_DIALOG_SIZE_M2
                    }
                } else {
                    INPUT_RANGE_LANDSCAPE_DIALOG_SIZE
                };
                elevation = if use_material3 {
                    date_picker_theme
                        .elevation
                        .or_else(|| defaults.elevation(app))
                        .expect("the defaults give an elevation")
                } else {
                    date_picker_theme
                        .elevation
                        .or(dialog_theme.elevation)
                        .unwrap_or(24.0)
                };
                shadow_color = date_picker_theme
                    .shadow_color
                    .or_else(|| defaults.shadow_color(app));
                surface_tint_color = date_picker_theme
                    .surface_tint_color
                    .or_else(|| defaults.surface_tint_color(app));
                shape = if use_material3 {
                    date_picker_theme
                        .shape
                        .clone()
                        .or_else(|| defaults.shape(app))
                } else {
                    date_picker_theme
                        .shape
                        .clone()
                        .or_else(|| dialog_theme.shape.clone())
                        .or_else(|| defaults.shape(app))
                };
                inset_padding = EdgeInsets::symmetric(24.0, 16.0);
            }
        }

        let mut dialog = Dialog::new()
            .inset_padding(inset_padding)
            .elevation(elevation)
            .clip_behavior(Clip::AntiAlias);
        if let Some(color) = date_picker_theme
            .background_color
            .or_else(|| defaults.background_color(app))
        {
            dialog = dialog.background_color(color);
        }
        if let Some(color) = shadow_color {
            dialog = dialog.shadow_color(color);
        }
        if let Some(color) = surface_tint_color {
            dialog = dialog.surface_tint_color(color);
        }
        if let Some(shape) = shape {
            dialog = dialog.shape(shape);
        }
        dialog
            .child(
                AnimatedContainer::new(DIALOG_SIZE_ANIMATION_DURATION)
                    .width(size.width())
                    .height(size.height())
                    .curve(Curves::ease_in())
                    .child(MediaQuery::with_clamped_text_scaling(
                        None,
                        0.0,
                        MAX_RANGE_TEXT_SCALE_FACTOR,
                        Builder::new(move |_app, _context| contents.clone()),
                    )),
            )
            .into_widget()
    }
}

const MONTH_SCROLL_DURATION: Duration = Duration::from_millis(200);

const MONTH_ITEM_HEADER_HEIGHT: f64 = 58.0;
const MONTH_ITEM_FOOTER_HEIGHT: f64 = 12.0;
const MONTH_ITEM_ROW_HEIGHT: f64 = 42.0;
const MONTH_ITEM_SPACE_BETWEEN_ROWS: f64 = 8.0;
const HORIZONTAL_PADDING: f64 = 8.0;
const MAX_CALENDAR_WIDTH_LANDSCAPE: f64 = 384.0;
const MAX_CALENDAR_WIDTH_PORTRAIT: f64 = 480.0;

/// Dart's `_CalendarRangePickerDialog`.
#[derive(Clone)]
struct CalendarRangePickerDialog {
    key: Option<KeyRef>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
    selected_start_date: Option<DateTime>,
    selected_end_date: Option<DateTime>,
    first_date: DateTime,
    last_date: DateTime,
    selectable_day_predicate: Option<SelectableDayForRangePredicate>,
    current_date: DateTime,
    on_start_date_changed: ValueChanged<Option<DateTime>>,
    on_end_date_changed: ValueChanged<Option<DateTime>>,
    on_confirm: Option<Listener>,
    on_cancel: Listener,
    entry_mode_button: Option<WidgetRef>,
    confirm_text: String,
    help_text: String,
}

impl fmt::Debug for CalendarRangePickerDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarRangePickerDialog")
            .field("selected_start_date", &self.selected_start_date)
            .field("selected_end_date", &self.selected_end_date)
            .finish_non_exhaustive()
    }
}

fn apply_color(style: Option<TextStyle>, color: Option<Color>) -> Option<TextStyle> {
    style.map(|style| match color {
        Some(color) => style.copy_with().color(color),
        None => style,
    })
}

impl StatelessWidget for CalendarRangePickerDialog {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let use_material3 = theme.use_material3;
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let orientation = MediaQuery::orientation_of(app, context);
        let theme_data = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let dialog_background = theme_data
            .range_picker_background_color
            .or_else(|| defaults.range_picker_background_color(app));
        let header_background = theme_data
            .range_picker_header_background_color
            .or_else(|| defaults.range_picker_header_background_color(app));
        let header_foreground = theme_data
            .range_picker_header_foreground_color
            .or_else(|| defaults.range_picker_header_foreground_color(app));
        #[allow(deprecated)]
        let header_disabled_foreground = header_foreground.map(|color| color.with_opacity(0.38));
        let headline_style = theme_data
            .range_picker_header_headline_style
            .clone()
            .or_else(|| defaults.range_picker_header_headline_style(app));
        let headline_help_style = apply_color(
            theme_data
                .range_picker_header_help_style
                .clone()
                .or_else(|| defaults.range_picker_header_help_style(app)),
            header_foreground,
        );
        let start_date_text = format_range_start_date(
            &*localizations,
            &*self.calendar_delegate,
            self.selected_start_date.as_ref(),
            self.selected_end_date.as_ref(),
        );
        let end_date_text = format_range_end_date(
            &*localizations,
            &*self.calendar_delegate,
            self.selected_start_date.as_ref(),
            self.selected_end_date.as_ref(),
            &self.calendar_delegate.now(),
        );
        let start_date_style = apply_color(
            headline_style.clone(),
            if self.selected_start_date.is_some() {
                header_foreground
            } else {
                header_disabled_foreground
            },
        );
        let end_date_style = apply_color(
            headline_style,
            if self.selected_end_date.is_some() {
                header_foreground
            } else {
                header_disabled_foreground
            },
        );
        let mut button_style = TextButton::style_from();
        if let Some(color) = header_foreground {
            button_style = button_style.foreground_color(color);
        }
        if let Some(color) = header_disabled_foreground {
            button_style = button_style.disabled_foreground_color(color);
        }
        let button_style = button_style.build();
        let mut icon_theme = IconThemeData::new();
        if let Some(color) = header_foreground {
            icon_theme = icon_theme.color(color);
        }

        let styled = |text: String, style: Option<TextStyle>| {
            let text = Text::new(text);
            match style {
                Some(style) => text.style(style),
                None => text,
            }
        };

        let mut actions: Vec<WidgetRef> = Vec::new();
        if orientation == Orientation::Landscape
            && let Some(entry_mode_button) = &self.entry_mode_button
        {
            actions.push(entry_mode_button.clone());
        }
        actions.push(
            TextButton::new(
                self.on_confirm.clone(),
                Some(Text::new(self.confirm_text.clone())),
            )
            .style(button_style)
            .into_widget(),
        );
        actions.push(SizedBox::new().width(8.0).into_widget());

        let mut bottom_row: Vec<WidgetRef> = vec![
            SizedBox::new()
                .width(if MediaQuery::width_of(app, context) < 360.0 {
                    42.0
                } else {
                    72.0
                })
                .into_widget(),
            Expanded::new(
                Semantics::new()
                    .label(format!(
                        "{} {start_date_text} to {end_date_text}",
                        self.help_text
                    ))
                    .exclude_semantics(true)
                    .child(
                        Column::new()
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .children([
                                styled(self.help_text.clone(), headline_help_style)
                                    .max_lines(1)
                                    .overflow(TextOverflow::Ellipsis)
                                    .into_widget(),
                                SizedBox::new().height(8.0).into_widget(),
                                waft_widgets::Row::new()
                                    .children([
                                        styled(start_date_text.clone(), start_date_style.clone())
                                            .max_lines(1)
                                            .overflow(TextOverflow::Ellipsis)
                                            .into_widget(),
                                        styled(" \u{2013} ".to_string(), start_date_style)
                                            .into_widget(),
                                        waft_widgets::Flexible::new(
                                            styled(end_date_text.clone(), end_date_style)
                                                .max_lines(1)
                                                .overflow(TextOverflow::Ellipsis),
                                        )
                                        .into_widget(),
                                    ])
                                    .into_widget(),
                                SizedBox::new().height(16.0).into_widget(),
                            ]),
                    ),
            )
            .into_widget(),
        ];
        if orientation == Orientation::Portrait
            && let Some(entry_mode_button) = &self.entry_mode_button
        {
            bottom_row.push(
                Padding::new(EdgeInsets::symmetric(0.0, 8.0).into())
                    .child(IconTheme::new(
                        icon_theme.clone(),
                        entry_mode_button.clone(),
                    ))
                    .into_widget(),
            );
        }

        let mut app_bar = AppBar::new()
            .icon_theme(icon_theme.clone())
            .actions_icon_theme(icon_theme)
            .leading(CloseButton::new().on_pressed(self.on_cancel.clone()))
            .actions(actions)
            .bottom(PreferredSizeWidgetRef::new(PreferredSize::new(
                Size::new(f64::INFINITY, 64.0),
                waft_widgets::Row::new().children(bottom_row),
            )));
        if use_material3 {
            app_bar = app_bar.elevation(0.0).scrolled_under_elevation(0.0);
        }
        if let Some(color) = header_background {
            app_bar = app_bar.background_color(color);
        }
        let mut scaffold = Scaffold::new()
            .app_bar(PreferredSizeWidgetRef::new(app_bar))
            .body(CalendarDateRangePicker {
                initial_start_date: self.selected_start_date,
                initial_end_date: self.selected_end_date,
                first_date: self.first_date,
                last_date: self.last_date,
                current_date: self.current_date,
                on_start_date_changed: Some(Rc::clone(&self.on_start_date_changed)),
                on_end_date_changed: Some(Rc::clone(&self.on_end_date_changed)),
                selectable_day_predicate: self.selectable_day_predicate.clone(),
                calendar_delegate: Rc::clone(&self.calendar_delegate),
            });
        if let Some(color) = dialog_background {
            scaffold = scaffold.background_color(color);
        }
        SafeArea::new(scaffold)
            .top(false)
            .left(false)
            .right(false)
            .into_widget()
    }
}

/// Dart's `_CalendarDateRangePicker`.
#[derive(Clone)]
struct CalendarDateRangePicker {
    initial_start_date: Option<DateTime>,
    initial_end_date: Option<DateTime>,
    first_date: DateTime,
    last_date: DateTime,
    selectable_day_predicate: Option<SelectableDayForRangePredicate>,
    current_date: DateTime,
    on_start_date_changed: Option<ValueChanged<Option<DateTime>>>,
    on_end_date_changed: Option<ValueChanged<Option<DateTime>>>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl fmt::Debug for CalendarDateRangePicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarDateRangePicker")
            .field("initial_start_date", &self.initial_start_date)
            .field("initial_end_date", &self.initial_end_date)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CalendarDateRangePicker {
    type State = CalendarDateRangePickerState;

    fn create_state(&self) -> CalendarDateRangePickerState {
        debug_assert!(
            self.initial_start_date
                .zip(self.initial_end_date)
                .is_none_or(|(start, end)| !start.is_after(&end)),
            "initialStartDate must be on or before initialEndDate."
        );
        debug_assert!(
            !self.last_date.is_before(&self.first_date),
            "firstDate must be on or before lastDate."
        );
        CalendarDateRangePickerState {
            state: StateData::new(),
            scroll_view_key: GlobalKey::new(),
            sliver_after_key: UniqueKey::new(),
            start_date: self.initial_start_date,
            end_date: self.initial_end_date,
            initial_month_index: 0,
            controller: None,
            show_week_bottom_divider: false,
        }
    }
}

/// Dart's `_CalendarDateRangePickerState`.
struct CalendarDateRangePickerState {
    state: StateData<CalendarDateRangePicker>,
    scroll_view_key: GlobalKey,
    sliver_after_key: UniqueKey,
    start_date: Option<DateTime>,
    end_date: Option<DateTime>,
    initial_month_index: i32,
    /// Created in `init_state`, because a state is created without the [`App`].
    controller: Option<Ref<ScrollController>>,
    show_week_bottom_divider: bool,
}

impl CalendarDateRangePickerState {
    fn controller(self: Handle<Self>, app: &App) -> Ref<ScrollController> {
        app.get(self)
            .controller
            .clone()
            .expect("created in init_state")
    }

    fn scroll_listener(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app).handle();
        let position = controller.position(app);
        let at_start = controller.offset(app) <= position.min_scroll_extent(app);
        if at_start {
            self.set_state(app, |_| {});
            app.get_mut(self).show_week_bottom_divider = false;
        } else if !app.get(self).show_week_bottom_divider {
            self.set_state(app, |_| {});
            app.get_mut(self).show_week_bottom_divider = true;
        }
    }

    fn number_of_months(self: Handle<Self>, app: &App) -> i32 {
        let widget = self.widget(app);
        widget
            .calendar_delegate
            .month_delta(&widget.first_date, &widget.last_date)
            + 1
    }

    fn vibrate(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        match Theme::of(app, context).platform {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => HapticFeedback::vibrate(app),
            TargetPlatform::IOS | TargetPlatform::MacOS => {}
        }
    }

    // This updates the selected date range using this logic:
    //
    // * From the unselected state, selecting one date creates the start date.
    //   * If the next selection is before the start date, reset date range and
    //     set the start date to that selection.
    //   * If the next selection is on or after the start date, set the end date
    //     to that selection.
    // * After both start and end dates are selected, any subsequent selection
    //   resets the date range and sets start date to that selection.
    fn update_selection(self: Handle<Self>, app: &mut App, date: DateTime) {
        self.vibrate(app);
        self.set_state(app, |_| {});
        let (start_date, end_date) = (app.get(self).start_date, app.get(self).end_date);
        let widget = self.widget(app).clone();
        if start_date.is_some_and(|start| !date.is_before(&start)) && end_date.is_none() {
            app.get_mut(self).end_date = Some(date);
            if let Some(on_end_date_changed) = &widget.on_end_date_changed {
                on_end_date_changed(app, Some(date));
            }
        } else {
            app.get_mut(self).start_date = Some(date);
            if let Some(on_start_date_changed) = &widget.on_start_date_changed {
                on_start_date_changed(app, Some(date));
            }
            if end_date.is_some() {
                app.get_mut(self).end_date = None;
                if let Some(on_end_date_changed) = &widget.on_end_date_changed {
                    on_end_date_changed(app, None);
                }
            }
        }
    }

    fn build_month_item(
        self: Handle<Self>,
        app: &mut App,
        index: i32,
        before_initial_month: bool,
    ) -> WidgetRef {
        let initial_month_index = app.get(self).initial_month_index;
        let month_index = if before_initial_month {
            initial_month_index - index - 1
        } else {
            initial_month_index + index
        };
        let widget = self.widget(app).clone();
        let month = widget
            .calendar_delegate
            .add_months_to_month_date(&widget.first_date, month_index);
        MonthItem {
            calendar_delegate: Rc::clone(&widget.calendar_delegate),
            selected_date_start: app.get(self).start_date,
            selected_date_end: app.get(self).end_date,
            current_date: widget.current_date,
            first_date: widget.first_date,
            last_date: widget.last_date,
            displayed_month: month,
            on_changed: Rc::new(move |app, date| self.update_selection(app, date)),
            selectable_day_predicate: widget.selectable_day_predicate.clone(),
        }
        .into_widget()
    }
}

impl fmt::Debug for CalendarDateRangePickerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarDateRangePickerState")
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .finish_non_exhaustive()
    }
}

impl State for CalendarDateRangePickerState {
    type Widget = CalendarDateRangePicker;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = ScrollController::new(app, 0.0, true, None, None, None);
        controller
            .handle()
            .add_listener(app, Listener::handle_method(self, Self::scroll_listener));
        app.get_mut(self).controller = Some(controller);

        // Calculate the index for the initially displayed month. This is needed to divide the
        // list of months into two `SliverList`s.
        let widget = self.widget(app).clone();
        let initial_date = widget.initial_start_date.unwrap_or(widget.current_date);
        let mut initial_month_index = 0;
        if !initial_date.is_before(&widget.first_date) && !initial_date.is_after(&widget.last_date)
        {
            initial_month_index = widget
                .calendar_delegate
                .month_delta(&widget.first_date, &initial_date);
        }
        let state = app.get_mut(self);
        state.initial_month_index = initial_month_index;
        state.show_week_bottom_divider = initial_month_index != 0;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.controller(app).handle().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let state = app.get(self);
        let (start_date, show_divider) = (state.start_date, state.show_week_bottom_divider);
        let scroll_view_key = state.scroll_view_key.clone();
        let sliver_after_key = state.sliver_after_key;
        let initial_month_index = state.initial_month_index;
        let number_of_months = self.number_of_months(app);
        let controller = self.controller(app);

        let mut children: Vec<WidgetRef> = vec![DayHeaders.into_widget()];
        if show_divider {
            children.push(Divider::new().height(0.0).into_widget());
        }
        children.push(
            Expanded::new(CalendarKeyboardNavigator {
                calendar_delegate: Rc::clone(&widget.calendar_delegate),
                first_date: widget.first_date,
                last_date: widget.last_date,
                initial_focused_day: start_date
                    .or(widget.initial_start_date)
                    .unwrap_or(widget.current_date),
                // In order to prevent performance issues when displaying the correct initial
                // month, 2 `SliverList`s are used to split the months. The first item in the
                // second SliverList is the initial month to be displayed.
                child: CustomScrollView::new()
                    .key(scroll_view_key.into())
                    .controller(controller.into_erased())
                    .center(sliver_after_key.into())
                    .slivers([
                        SliverList::builder(move |app, _context, index| {
                            if index >= initial_month_index {
                                return None;
                            }
                            Some(self.build_month_item(app, index, true))
                        })
                        .into_widget(),
                        SliverList::builder(move |app, _context, index| {
                            if index >= number_of_months - initial_month_index {
                                return None;
                            }
                            Some(self.build_month_item(app, index, false))
                        })
                        .key(sliver_after_key.into())
                        .into_widget(),
                    ])
                    .into_widget(),
            })
            .into_widget(),
        );
        Column::new().children(children).into_widget()
    }
}

/// Dart's `_CalendarKeyboardNavigator`.
#[derive(Clone)]
struct CalendarKeyboardNavigator {
    child: WidgetRef,
    first_date: DateTime,
    last_date: DateTime,
    initial_focused_day: DateTime,
    calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl fmt::Debug for CalendarKeyboardNavigator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarKeyboardNavigator")
            .field("initial_focused_day", &self.initial_focused_day)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CalendarKeyboardNavigator {
    type State = CalendarKeyboardNavigatorState;

    fn create_state(&self) -> CalendarKeyboardNavigatorState {
        CalendarKeyboardNavigatorState {
            state: StateData::new(),
            shortcut_map: Vec::new(),
            action_map: HashMap::new(),
            day_grid_focus: None,
            day_traversal_direction: None,
            focused_day: None,
        }
    }
}

struct CalendarKeyboardNavigatorState {
    state: StateData<CalendarKeyboardNavigator>,
    shortcut_map: ShortcutMap,
    action_map: HashMap<TypeId, ActionRef>,
    day_grid_focus: Option<Ref<FocusNode>>,
    day_traversal_direction: Option<TraversalDirection>,
    focused_day: Option<DateTime>,
}

fn directional_shortcut(
    key: LogicalKeyboardKey,
    direction: TraversalDirection,
) -> (ShortcutActivatorRef, IntentRef) {
    (
        Rc::new(SingleActivator::new(key)) as ShortcutActivatorRef,
        Rc::new(DirectionalFocusIntent::new(direction)) as IntentRef,
    )
}

fn direction_offset(direction: TraversalDirection) -> i32 {
    match direction {
        TraversalDirection::Up => -DateTime::DAYS_PER_WEEK,
        TraversalDirection::Right => 1,
        TraversalDirection::Down => DateTime::DAYS_PER_WEEK,
        TraversalDirection::Left => -1,
    }
}

impl CalendarKeyboardNavigatorState {
    fn day_grid_focus(self: Handle<Self>, app: &App) -> Ref<FocusNode> {
        app.get(self)
            .day_grid_focus
            .clone()
            .expect("created in init_state")
    }

    fn handle_grid_focus_change(self: Handle<Self>, app: &mut App, focused: bool) {
        self.set_state(app, |_| {});
        if focused && app.get(self).focused_day.is_none() {
            let initial_focused_day = self.widget(app).initial_focused_day;
            app.get_mut(self).focused_day = Some(initial_focused_day);
        }
    }

    fn handle_grid_next_focus(self: Handle<Self>, app: &mut App) {
        let day_grid_focus = self.day_grid_focus(app).handle();
        day_grid_focus.request_focus(app, None);
        day_grid_focus.as_node().next_focus(app);
    }

    fn handle_grid_previous_focus(self: Handle<Self>, app: &mut App) {
        let day_grid_focus = self.day_grid_focus(app).handle();
        day_grid_focus.request_focus(app, None);
        day_grid_focus.as_node().previous_focus(app);
    }

    fn handle_direction_focus(self: Handle<Self>, app: &mut App, direction: TraversalDirection) {
        let focused_day = app
            .get(self)
            .focused_day
            .expect("a focused day when navigating by direction");
        self.set_state(app, |_| {});
        if let Some(next_date) = self.next_date_in_direction(app, &focused_day, direction) {
            let state = app.get_mut(self);
            state.focused_day = Some(next_date);
            state.day_traversal_direction = Some(direction);
        }
    }

    fn day_direction_offset(
        traversal_direction: TraversalDirection,
        text_direction: TextDirection,
    ) -> i32 {
        let mut traversal_direction = traversal_direction;
        // Swap left and right if the text direction if RTL
        if text_direction == TextDirection::Rtl {
            if traversal_direction == TraversalDirection::Left {
                traversal_direction = TraversalDirection::Right;
            } else if traversal_direction == TraversalDirection::Right {
                traversal_direction = TraversalDirection::Left;
            }
        }
        direction_offset(traversal_direction)
    }

    fn next_date_in_direction(
        self: Handle<Self>,
        app: &mut App,
        date: &DateTime,
        direction: TraversalDirection,
    ) -> Option<DateTime> {
        let context = self.context(app);
        let text_direction = Directionality::of(app, context);
        let widget = self.widget(app);
        let next_date = widget
            .calendar_delegate
            .add_days_to_date(date, Self::day_direction_offset(direction, text_direction));
        if !next_date.is_before(&widget.first_date) && !next_date.is_after(&widget.last_date) {
            return Some(next_date);
        }
        None
    }
}

impl fmt::Debug for CalendarKeyboardNavigatorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarKeyboardNavigatorState")
            .field("focused_day", &self.focused_day)
            .finish_non_exhaustive()
    }
}

impl State for CalendarKeyboardNavigatorState {
    type Widget = CalendarKeyboardNavigator;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let shortcut_map: ShortcutMap = vec![
            directional_shortcut(LogicalKeyboardKey::ARROW_LEFT, TraversalDirection::Left),
            directional_shortcut(LogicalKeyboardKey::ARROW_RIGHT, TraversalDirection::Right),
            directional_shortcut(LogicalKeyboardKey::ARROW_DOWN, TraversalDirection::Down),
            directional_shortcut(LogicalKeyboardKey::ARROW_UP, TraversalDirection::Up),
        ];
        let next_focus = CallbackAction::<NextFocusIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &NextFocusIntent| {
                self.handle_grid_next_focus(app);
                None
            }),
        );
        let previous_focus = CallbackAction::<PreviousFocusIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &PreviousFocusIntent| {
                self.handle_grid_previous_focus(app);
                None
            }),
        );
        let directional_focus = CallbackAction::<DirectionalFocusIntent>::new(
            app,
            Rc::new(move |app: &mut App, intent: &DirectionalFocusIntent| {
                self.handle_direction_focus(app, intent.direction);
                None
            }),
        );
        let mut action_map = HashMap::new();
        action_map.insert(TypeId::of::<NextFocusIntent>(), next_focus.into_erased());
        action_map.insert(
            TypeId::of::<PreviousFocusIntent>(),
            previous_focus.into_erased(),
        );
        action_map.insert(
            TypeId::of::<DirectionalFocusIntent>(),
            directional_focus.into_erased(),
        );
        let day_grid_focus = FocusNode::new(app);
        day_grid_focus
            .handle()
            .set_debug_label(app, Some("Day Grid".to_string()));
        let state = app.get_mut(self);
        state.shortcut_map = shortcut_map;
        state.action_map = action_map;
        state.day_grid_focus = Some(day_grid_focus);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.day_grid_focus(app).handle().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let day_grid_focus = self.day_grid_focus(app);
        let has_focus = day_grid_focus.handle().has_focus(app);
        let state = app.get(self);
        let (shortcuts, actions) = (state.shortcut_map.clone(), state.action_map.clone());
        let (date, scroll_direction) = (
            if has_focus { state.focused_day } else { None },
            if has_focus {
                state.day_traversal_direction
            } else {
                None
            },
        );
        FocusableActionDetector::new(FocusedDate {
            child: widget.child.clone(),
            calendar_delegate: Rc::clone(&widget.calendar_delegate),
            date,
            scroll_direction,
        })
        .shortcuts(shortcuts)
        .actions(actions)
        .focus_node(day_grid_focus.into_erased())
        .on_focus_change(move |app, focused| self.handle_grid_focus_change(app, focused))
        .into_widget()
    }
}

/// Dart's `_FocusedDate` of date_picker.dart.
#[derive(Debug)]
struct FocusedDate {
    child: WidgetRef,
    calendar_delegate: Rc<dyn CalendarDelegate>,
    date: Option<DateTime>,
    scroll_direction: Option<TraversalDirection>,
}

impl FocusedDate {
    fn maybe_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<(Option<DateTime>, Option<TraversalDirection>)> {
        context
            .depend_on_inherited_widget_of_exact_type::<FocusedDate>(app)
            .map(|focused| (focused.date, focused.scroll_direction))
    }
}

impl InheritedWidget for FocusedDate {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn update_should_notify(&self, old_widget: &FocusedDate) -> bool {
        !self
            .calendar_delegate
            .is_same_day(self.date.as_ref(), old_widget.date.as_ref())
            || self.scroll_direction != old_widget.scroll_direction
    }
}

/// Dart's `_DayHeaders`.
#[derive(Clone, Copy, Debug)]
struct DayHeaders;

impl StatelessWidget for DayHeaders {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme_data = Theme::of(app, context);
        let color_scheme = &theme_data.color_scheme;
        let text_style = apply_color(
            theme_data.text_theme.title_small.clone(),
            Some(color_scheme.on_surface),
        )
        .expect("the text theme has titleSmall");
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let narrow_weekdays = localizations.narrow_weekdays();
        let mut labels: Vec<WidgetRef> = vec![SizedBox::shrink().into_widget()];
        let mut i = localizations.first_day_of_week_index();
        while labels.len() < DateTime::DAYS_PER_WEEK as usize + 1 {
            labels.push(
                ExcludeSemantics::new()
                    .child(Center::new().child(
                        Text::new(narrow_weekdays[i as usize].clone()).style(text_style.clone()),
                    ))
                    .into_widget(),
            );
            i = (i + 1) % DateTime::DAYS_PER_WEEK;
        }
        labels.push(SizedBox::shrink().into_widget());

        ConstrainedBox::new(BoxConstraints {
            max_width: if MediaQuery::orientation_of(app, context) == Orientation::Landscape {
                MAX_CALENDAR_WIDTH_LANDSCAPE
            } else {
                MAX_CALENDAR_WIDTH_PORTRAIT
            },
            max_height: MONTH_ITEM_ROW_HEIGHT,
            ..BoxConstraints::new()
        })
        .child(
            GridView::custom(
                Rc::new(MonthItemGridDelegate),
                SliverChildListDelegate::new(labels)
                    .add_repaint_boundaries(false)
                    .into_delegate(),
            )
            .shrink_wrap(true),
        )
        .into_widget()
    }
}

/// Dart's `_MonthItemGridDelegate`.
#[derive(Clone, Copy, Debug)]
struct MonthItemGridDelegate;

impl SliverGridDelegate for MonthItemGridDelegate {
    fn get_layout(&self, constraints: SliverConstraints) -> Box<dyn SliverGridLayout> {
        let tile_width = ((constraints.cross_axis_extent - 2.0 * HORIZONTAL_PADDING)
            / f64::from(DateTime::DAYS_PER_WEEK))
        .max(0.0);
        Box::new(MonthSliverGridLayout {
            cross_axis_count: DateTime::DAYS_PER_WEEK + 2,
            day_child_width: tile_width,
            edge_child_width: HORIZONTAL_PADDING,
            reverse_cross_axis: axis_direction_is_reversed(constraints.cross_axis_direction),
        })
    }

    fn should_relayout(&self, _old_delegate: &dyn SliverGridDelegate) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Dart's `_MonthSliverGridLayout`.
#[derive(Clone, Copy, Debug)]
struct MonthSliverGridLayout {
    cross_axis_count: i32,
    day_child_width: f64,
    edge_child_width: f64,
    reverse_cross_axis: bool,
}

impl MonthSliverGridLayout {
    fn row_height() -> f64 {
        MONTH_ITEM_ROW_HEIGHT + MONTH_ITEM_SPACE_BETWEEN_ROWS
    }

    fn child_height() -> f64 {
        MONTH_ITEM_ROW_HEIGHT
    }

    fn cross_axis_offset(&self, cross_axis_start: f64, is_padding: bool) -> f64 {
        if self.reverse_cross_axis {
            return (f64::from(self.cross_axis_count - 2) * self.day_child_width
                + 2.0 * self.edge_child_width)
                - cross_axis_start
                - if is_padding {
                    self.edge_child_width
                } else {
                    self.day_child_width
                };
        }
        cross_axis_start
    }
}

impl SliverGridLayout for MonthSliverGridLayout {
    fn get_min_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32 {
        self.cross_axis_count * (scroll_offset / Self::row_height()).trunc() as i32
    }

    fn get_max_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32 {
        let main_axis_count = (scroll_offset / Self::row_height()).ceil() as i32;
        0.max(self.cross_axis_count * main_axis_count - 1)
    }

    fn get_geometry_for_child_index(&self, index: i32) -> SliverGridGeometry {
        let adjusted_index = index % self.cross_axis_count;
        let is_edge = adjusted_index == 0 || adjusted_index == self.cross_axis_count - 1;
        let cross_axis_start = 0.0_f64
            .max(f64::from(adjusted_index - 1) * self.day_child_width + self.edge_child_width);
        SliverGridGeometry {
            scroll_offset: f64::from(index / self.cross_axis_count) * Self::row_height(),
            cross_axis_offset: self.cross_axis_offset(cross_axis_start, is_edge),
            main_axis_extent: Self::child_height(),
            cross_axis_extent: if is_edge {
                self.edge_child_width
            } else {
                self.day_child_width
            },
        }
    }

    fn compute_max_scroll_offset(&self, child_count: i32) -> f64 {
        debug_assert!(child_count >= 0);
        let main_axis_count = ((child_count - 1) / self.cross_axis_count) + 1;
        let main_axis_spacing = Self::row_height() - Self::child_height();
        Self::row_height() * f64::from(main_axis_count) - main_axis_spacing
    }
}

/// Dart's `_MonthItem`.
#[derive(Clone)]
struct MonthItem {
    selected_date_start: Option<DateTime>,
    selected_date_end: Option<DateTime>,
    current_date: DateTime,
    on_changed: ValueChanged<DateTime>,
    first_date: DateTime,
    last_date: DateTime,
    displayed_month: DateTime,
    selectable_day_predicate: Option<SelectableDayForRangePredicate>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl fmt::Debug for MonthItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonthItem")
            .field("displayed_month", &self.displayed_month)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MonthItem {
    type State = MonthItemState;

    fn create_state(&self) -> MonthItemState {
        debug_assert!(!self.first_date.is_after(&self.last_date));
        debug_assert!(
            self.selected_date_start
                .zip(self.selected_date_end)
                .is_none_or(|(start, end)| !start.is_after(&end))
        );
        MonthItemState {
            state: StateData::new(),
            day_focus_nodes: Vec::new(),
        }
    }
}

struct MonthItemState {
    state: StateData<MonthItem>,
    /// Created in `init_state`, because a state is created without the [`App`].
    day_focus_nodes: Vec<Ref<FocusNode>>,
}

impl MonthItemState {
    fn highlight_color(self: Handle<Self>, app: &mut App, context: BuildContext) -> Color {
        DatePickerTheme::of(app, context)
            .range_selection_background_color
            .or_else(|| {
                DatePickerTheme::defaults(app, context).range_selection_background_color(app)
            })
            .expect("the defaults give a range selection background color")
    }

    fn day_focus_changed(self: Handle<Self>, app: &mut App, focused: bool) {
        if !focused {
            return;
        }
        let context = self.context(app);
        let Some((_, Some(focus_direction))) = FocusedDate::maybe_of(app, context) else {
            return;
        };
        let policy = match focus_direction {
            TraversalDirection::Up | TraversalDirection::Left => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtStart
            }
            TraversalDirection::Right | TraversalDirection::Down => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
            }
        };
        let primary_focus = waft_widgets::FocusManager::instance(app).primary_focus(app);
        if let Some(primary_focus_context) = primary_focus.and_then(|node| node.context(app)) {
            Scrollable::ensure_visible(
                app,
                primary_focus_context,
                0.0,
                MONTH_SCROLL_DURATION,
                Curves::linear(),
                policy,
            );
        }
    }

    fn build_day_item(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        day_to_build: DateTime,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let day = day_to_build.day();
        let is_disabled = day_to_build.is_after(&widget.last_date)
            || day_to_build.is_before(&widget.first_date)
            || widget
                .selectable_day_predicate
                .as_ref()
                .is_some_and(|predicate| {
                    !predicate(
                        day_to_build,
                        widget.selected_date_start,
                        widget.selected_date_end,
                    )
                });
        let is_range_selected =
            widget.selected_date_start.is_some() && widget.selected_date_end.is_some();
        let is_selected_day_start = widget
            .selected_date_start
            .is_some_and(|start| day_to_build.is_at_same_moment_as(&start));
        let is_selected_day_end = widget
            .selected_date_end
            .is_some_and(|end| day_to_build.is_at_same_moment_as(&end));
        let is_in_range = match (widget.selected_date_start, widget.selected_date_end) {
            (Some(start), Some(end)) => {
                day_to_build.is_after(&start) && day_to_build.is_before(&end)
            }
            _ => false,
        };
        let is_one_day_range =
            is_range_selected && widget.selected_date_start == widget.selected_date_end;
        let is_today = widget
            .calendar_delegate
            .is_same_day(Some(&widget.current_date), Some(&day_to_build));
        let highlight_color = self.highlight_color(app, context);
        let focus_node = app.get(self).day_focus_nodes[(day - 1) as usize].clone();

        DayItem {
            calendar_delegate: Rc::clone(&widget.calendar_delegate),
            day: day_to_build,
            focus_node,
            on_changed: Rc::clone(&widget.on_changed),
            on_focus_change: Rc::new(move |app, focused| self.day_focus_changed(app, focused)),
            highlight_color,
            is_disabled,
            is_range_selected,
            is_selected_day_start,
            is_selected_day_end,
            is_in_range,
            is_one_day_range,
            is_today,
        }
        .into_widget()
    }

    fn build_edge_box(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        is_highlighted: bool,
    ) -> WidgetRef {
        let empty = LimitedBox::new()
            .max_width(0.0)
            .max_height(0.0)
            .child(SizedBox::expand())
            .into_widget();
        if is_highlighted {
            ColoredBox::new(self.highlight_color(app, context))
                .child(empty)
                .into_widget()
        } else {
            empty
        }
    }
}

impl fmt::Debug for MonthItemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MonthItemState")
    }
}

impl State for MonthItemState {
    type Widget = MonthItem;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let days_in_month = widget.calendar_delegate.get_days_in_month(
            widget.displayed_month.year(),
            widget.displayed_month.month(),
        );
        let nodes: Vec<Ref<FocusNode>> = (0..days_in_month)
            .map(|index| {
                let node = FocusNode::new(app);
                node.handle().set_skip_traversal(app, true);
                node.handle()
                    .set_debug_label(app, Some(format!("Day {}", index + 1)));
                node
            })
            .collect();
        app.get_mut(self).day_focus_nodes = nodes;
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        // Check to see if the focused date is in this month, if so focus it.
        let context = self.context(app);
        let focused_date = FocusedDate::maybe_of(app, context).and_then(|(date, _)| date);
        let widget = self.widget(app).clone();
        if let Some(focused_date) = focused_date
            && widget
                .calendar_delegate
                .is_same_month(Some(&widget.displayed_month), Some(&focused_date))
        {
            let node = app.get(self).day_focus_nodes[(focused_date.day() - 1) as usize].clone();
            node.handle().request_focus(app, None);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for node in app.get(self).day_focus_nodes.clone() {
            node.handle().dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme_data = Theme::of(app, context);
        let text_theme = theme_data.text_theme.clone();
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let widget = self.widget(app).clone();
        let delegate = Rc::clone(&widget.calendar_delegate);
        let year = widget.displayed_month.year();
        let month = widget.displayed_month.month();
        let days_in_month = delegate.get_days_in_month(year, month);
        let day_offset = delegate.first_day_offset(year, month, &*localizations);
        let weeks = ((days_in_month + day_offset) as f64 / f64::from(DateTime::DAYS_PER_WEEK))
            .ceil() as i32;
        let grid_height = f64::from(weeks) * MONTH_ITEM_ROW_HEIGHT
            + f64::from(weeks - 1) * MONTH_ITEM_SPACE_BETWEEN_ROWS;
        let empty = || {
            LimitedBox::new()
                .max_width(0.0)
                .max_height(0.0)
                .child(SizedBox::expand())
                .into_widget()
        };
        let mut day_items: Vec<WidgetRef> = Vec::new();

        // 1-based day of month, e.g. 1-31 for January, and 1-29 for February on a leap year.
        for day in (1 - day_offset)..=days_in_month {
            if day < 1 {
                day_items.push(empty());
            } else {
                let day_to_build = delegate.get_day(year, month, day);
                day_items.push(self.build_day_item(app, context, day_to_build));
            }
        }

        // Add the leading/trailing edge containers to each week in order to correctly extend
        // the range highlight.
        let mut padded_day_items: Vec<WidgetRef> = Vec::new();
        for i in 0..weeks {
            let start = (i * DateTime::DAYS_PER_WEEK) as usize;
            let end = ((start as i32 + DateTime::DAYS_PER_WEEK) as usize).min(day_items.len());
            let mut week_list: Vec<WidgetRef> = day_items[start..end].to_vec();

            let date_after_leading_padding =
                delegate.get_day(year, month, start as i32 - day_offset + 1);
            // Only color the edge container if it is after the start date and on/before the
            // end date.
            let is_leading_in_range = !(day_offset > 0 && i == 0)
                && match (widget.selected_date_start, widget.selected_date_end) {
                    (Some(range_start), Some(range_end)) => {
                        date_after_leading_padding.is_after(&range_start)
                            && !date_after_leading_padding.is_after(&range_end)
                    }
                    _ => false,
                };
            week_list.insert(0, self.build_edge_box(app, context, is_leading_in_range));

            // Only add a trailing edge container if it is for a full week and not a partial
            // week.
            if end < day_items.len()
                || (end == day_items.len()
                    && day_items
                        .len()
                        .is_multiple_of(DateTime::DAYS_PER_WEEK as usize))
            {
                let date_before_trailing_padding =
                    delegate.get_day(year, month, end as i32 - day_offset);
                // Only color the edge container if it is on/after the start date and before
                // the end date.
                let is_trailing_in_range =
                    match (widget.selected_date_start, widget.selected_date_end) {
                        (Some(range_start), Some(range_end)) => {
                            !date_before_trailing_padding.is_before(&range_start)
                                && date_before_trailing_padding.is_before(&range_end)
                        }
                        _ => false,
                    };
                week_list.push(self.build_edge_box(app, context, is_trailing_in_range));
            }

            padded_day_items.extend(week_list);
        }

        let max_width = if MediaQuery::orientation_of(app, context) == Orientation::Landscape {
            MAX_CALENDAR_WIDTH_LANDSCAPE
        } else {
            MAX_CALENDAR_WIDTH_PORTRAIT
        };
        let month_style = apply_color(
            text_theme.body_medium.clone(),
            Some(theme_data.color_scheme.on_surface),
        )
        .expect("the text theme has bodyMedium");
        Column::new()
            .children([
                ConstrainedBox::new(BoxConstraints {
                    max_width,
                    min_height: MONTH_ITEM_HEADER_HEIGHT,
                    max_height: MONTH_ITEM_HEADER_HEIGHT,
                    ..BoxConstraints::new()
                })
                .child(
                    Padding::new(EdgeInsets::symmetric(0.0, 16.0).into()).child(
                        Align::new()
                            .alignment(AlignmentGeometry::from(AlignmentDirectional::CENTER_START))
                            .child(
                                ExcludeSemantics::new().child(
                                    Text::new(delegate.format_month_year(
                                        &widget.displayed_month,
                                        &*localizations,
                                    ))
                                    .style(month_style),
                                ),
                            ),
                    ),
                )
                .into_widget(),
                ConstrainedBox::new(BoxConstraints {
                    max_width,
                    max_height: grid_height,
                    ..BoxConstraints::new()
                })
                .child(
                    GridView::custom(
                        Rc::new(MonthItemGridDelegate),
                        SliverChildListDelegate::new(padded_day_items)
                            .add_repaint_boundaries(false)
                            .into_delegate(),
                    )
                    .physics(Rc::new(waft_widgets::NeverScrollableScrollPhysics::new())),
                )
                .into_widget(),
                SizedBox::new()
                    .height(MONTH_ITEM_FOOTER_HEIGHT)
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// Dart's `_DayItem`.
#[derive(Clone)]
struct DayItem {
    day: DateTime,
    focus_node: Ref<FocusNode>,
    on_changed: ValueChanged<DateTime>,
    on_focus_change: ValueChanged<bool>,
    highlight_color: Color,
    is_disabled: bool,
    is_range_selected: bool,
    is_selected_day_start: bool,
    is_selected_day_end: bool,
    is_in_range: bool,
    is_one_day_range: bool,
    is_today: bool,
    calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl fmt::Debug for DayItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DayItem")
            .field("day", &self.day)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for DayItem {
    type State = DayItemState;

    fn create_state(&self) -> DayItemState {
        DayItemState {
            state: StateData::new(),
            states_controller: None,
        }
    }
}

struct DayItemState {
    state: StateData<DayItem>,
    /// Created in `init_state`, because a state is created without the [`App`].
    states_controller: Option<Ref<WidgetStatesController>>,
}

impl DayItemState {
    fn states_controller(self: Handle<Self>, app: &App) -> Ref<WidgetStatesController> {
        app.get(self)
            .states_controller
            .clone()
            .expect("created in init_state")
    }
}

impl fmt::Debug for DayItemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DayItemState")
    }
}

impl State for DayItemState {
    type Widget = DayItem;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = WidgetStatesController::new(app, None);
        app.get_mut(self).states_controller = Some(controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.states_controller(app).handle().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let color_scheme = theme.color_scheme.clone();
        let text_theme = theme.text_theme.clone();
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let text_direction = Directionality::of(app, context);
        let widget = self.widget(app).clone();
        let highlight_color = widget.highlight_color;

        let mut decoration: Option<ShapeDecoration> = None;
        let mut item_style = text_theme.body_medium.clone();

        let mut states = WidgetStates::new();
        if widget.is_disabled {
            states.insert(waft_widgets::WidgetState::Disabled);
        }
        if widget.is_selected_day_start || widget.is_selected_day_end {
            states.insert(waft_widgets::WidgetState::Selected);
        }
        let states_controller = self.states_controller(app);
        states_controller.handle().set_value(app, states.clone());

        let resolve_color =
            |property: Option<waft_widgets::WidgetStatePropertyRef<Option<Color>>>,
             default: Option<waft_widgets::WidgetStatePropertyRef<Option<Color>>>,
             states: &WidgetStates| {
                property
                    .and_then(|property| property.resolve(states))
                    .or_else(|| default.and_then(|property| property.resolve(states)))
            };
        let day_foreground_color = resolve_color(
            date_picker_theme.day_foreground_color.clone(),
            defaults.day_foreground_color(app),
            &states,
        );
        let day_background_color = resolve_color(
            date_picker_theme.day_background_color.clone(),
            defaults.day_background_color(app),
            &states,
        );
        let is_in_range = widget.is_in_range;
        let (theme_range_overlay, defaults_range_overlay) = (
            date_picker_theme.range_selection_overlay_color.clone(),
            defaults.range_selection_overlay_color(app),
        );
        let (theme_day_overlay, defaults_day_overlay) = (
            date_picker_theme.day_overlay_color.clone(),
            defaults.day_overlay_color(app),
        );
        let day_overlay_color =
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
                let (theme_property, default_property) = if is_in_range {
                    (theme_range_overlay.clone(), defaults_range_overlay.clone())
                } else {
                    (theme_day_overlay.clone(), defaults_day_overlay.clone())
                };
                theme_property
                    .and_then(|property| property.resolve(states))
                    .or_else(|| default_property.and_then(|property| property.resolve(states)))
            });
        let day_shape: OutlinedBorderRef = date_picker_theme
            .day_shape
            .clone()
            .and_then(|property| property.resolve(&states))
            .or_else(|| {
                defaults
                    .day_shape(app)
                    .and_then(|property| property.resolve(&states))
            })
            .unwrap_or_else(|| OutlinedBorderRef::new(CircleBorder::default()));

        let mut highlight_painter: Option<HighlightPainter> = None;
        if widget.is_selected_day_start || widget.is_selected_day_end {
            // The selected start and end dates get a custom shaped background highlight, and a
            // contrasting text color.
            item_style = apply_color(item_style, day_foreground_color);
            let mut shape_decoration =
                ShapeDecoration::new(ShapeBorderRef::from(day_shape.clone()));
            if let Some(color) = day_background_color {
                shape_decoration = shape_decoration.color(color);
            }
            decoration = Some(shape_decoration);

            if widget.is_range_selected && !widget.is_one_day_range {
                let style = if widget.is_selected_day_start {
                    HighlightPainterStyle::HighlightTrailing
                } else {
                    HighlightPainterStyle::HighlightLeading
                };
                highlight_painter = Some(HighlightPainter {
                    color: highlight_color,
                    style,
                    text_direction: Some(text_direction),
                });
            }
        } else if widget.is_in_range {
            // The days within the range get a light background highlight.
            highlight_painter = Some(HighlightPainter {
                color: highlight_color,
                style: HighlightPainterStyle::HighlightAll,
                text_direction: Some(text_direction),
            });
            if widget.is_disabled {
                #[allow(deprecated)]
                let disabled = color_scheme.on_surface.with_opacity(0.38);
                item_style = apply_color(item_style, Some(disabled));
            }
        } else if widget.is_disabled {
            #[allow(deprecated)]
            let disabled = color_scheme.on_surface.with_opacity(0.38);
            item_style = apply_color(item_style, Some(disabled));
        } else if widget.is_today {
            // The current day gets a different text color and a custom shape border.
            item_style = apply_color(item_style, Some(color_scheme.primary));
            let today_side: BorderSide = date_picker_theme
                .today_border
                .or_else(|| defaults.today_border(app))
                .expect("the defaults give a today border")
                .copy_with(Some(color_scheme.primary), None, None, None);
            decoration = Some(ShapeDecoration::new(ShapeBorderRef::from(
                day_shape.copy_with(Some(today_side)),
            )));
        }

        let day_text = localizations.format_decimal(i64::from(widget.day.day()));

        // We want the day of month to be spoken first irrespective of the locale-specific
        // preferences or TextDirection. This is because an accessibility user is more likely
        // to be interested in the day of month before the rest of the date, as they are
        // looking for the day of month. To do that we prepend day of month to the formatted
        // full date.
        let semantic_label_suffix = if widget.is_today {
            format!(", {}", localizations.current_date_label())
        } else {
            String::new()
        };
        let mut semantic_label = format!(
            "{day_text}, {}{semantic_label_suffix}",
            widget
                .calendar_delegate
                .format_full_date(&widget.day, &*localizations)
        );
        if widget.is_selected_day_start {
            semantic_label = localizations.date_range_start_date_semantic_label(&semantic_label);
        } else if widget.is_selected_day_end {
            semantic_label = localizations.date_range_end_date_semantic_label(&semantic_label);
        }

        let mut text = Text::new(day_text);
        if let Some(item_style) = item_style {
            text = text.style(item_style);
        }
        let mut container = Container::new()
            .alignment(AlignmentGeometry::from(Alignment::CENTER))
            .child(
                Semantics::new()
                    .label(semantic_label)
                    .selected(widget.is_selected_day_start || widget.is_selected_day_end)
                    .child(ExcludeSemantics::new().child(text)),
            );
        if let Some(decoration) = decoration {
            container = container.decoration(decoration);
        }
        let mut day_widget = container.into_widget();

        if let Some(highlight_painter) = highlight_painter {
            day_widget = CustomPaint::new()
                .painter(highlight_painter)
                .child(day_widget)
                .into_widget();
        }

        if !widget.is_disabled {
            let (on_changed, day) = (Rc::clone(&widget.on_changed), widget.day);
            let on_focus_change = Rc::clone(&widget.on_focus_change);
            day_widget = InkResponse::new()
                .focus_node(widget.focus_node.clone().into_erased())
                .on_tap(Listener::new(move |app: &mut App| on_changed(app, day)))
                .custom_border(day_shape)
                .contained_ink_well(true)
                .states_controller(states_controller)
                .overlay_color(day_overlay_color)
                .on_focus_change(on_focus_change)
                .child(day_widget)
                .into_widget();
        }

        day_widget
    }
}

/// Dart's `_HighlightPainterStyle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HighlightPainterStyle {
    /// Paint nothing.
    None,

    /// Paint a rectangle that occupies the leading half of the space.
    HighlightLeading,

    /// Paint a rectangle that occupies the trailing half of the space.
    HighlightTrailing,

    /// Paint a rectangle that occupies all available space.
    HighlightAll,
}

/// Dart's `_HighlightPainter`.
#[derive(Clone, Copy, Debug)]
struct HighlightPainter {
    color: Color,
    style: HighlightPainterStyle,
    text_direction: Option<TextDirection>,
}

impl CustomPainter for HighlightPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        if self.style == HighlightPainterStyle::None {
            return;
        }

        let paint = Paint::from_color(self.color.into());

        let rtl = match self.text_direction {
            Some(TextDirection::Rtl) | None => true,
            Some(TextDirection::Ltr) => false,
        };

        match self.style {
            HighlightPainterStyle::HighlightLeading if rtl => {
                canvas.draw_rect(
                    Rect::from_ltwh(size.width() / 2.0, 0.0, size.width() / 2.0, size.height()),
                    &paint,
                );
            }
            HighlightPainterStyle::HighlightTrailing if !rtl => {
                canvas.draw_rect(
                    Rect::from_ltwh(size.width() / 2.0, 0.0, size.width() / 2.0, size.height()),
                    &paint,
                );
            }
            HighlightPainterStyle::HighlightLeading | HighlightPainterStyle::HighlightTrailing => {
                canvas.draw_rect(
                    Rect::from_ltwh(0.0, 0.0, size.width() / 2.0, size.height()),
                    &paint,
                );
            }
            HighlightPainterStyle::HighlightAll => {
                canvas.draw_rect(
                    Rect::from_ltwh(0.0, 0.0, size.width(), size.height()),
                    &paint,
                );
            }
            HighlightPainterStyle::None => {}
        }
    }

    fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Dart's `_InputDateRangePickerDialog`.
#[derive(Clone)]
struct InputDateRangePickerDialog {
    calendar_delegate: Rc<dyn CalendarDelegate>,
    selected_start_date: Option<DateTime>,
    selected_end_date: Option<DateTime>,
    current_date: DateTime,
    picker: WidgetRef,
    on_confirm: Listener,
    on_cancel: Listener,
    entry_mode_button: Option<WidgetRef>,
    confirm_text: String,
    cancel_text: String,
    help_text: String,
}

impl fmt::Debug for InputDateRangePickerDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputDateRangePickerDialog")
            .field("selected_start_date", &self.selected_start_date)
            .field("selected_end_date", &self.selected_end_date)
            .finish_non_exhaustive()
    }
}

impl InputDateRangePickerDialog {
    fn format_date_range(
        &self,
        app: &mut App,
        context: BuildContext,
        start: Option<&DateTime>,
        end: Option<&DateTime>,
        now: &DateTime,
    ) -> String {
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let start_text =
            format_range_start_date(&*localizations, &*self.calendar_delegate, start, end);
        let end_text =
            format_range_end_date(&*localizations, &*self.calendar_delegate, start, end, now);
        if start.is_none() || end.is_none() {
            return localizations.unspecified_date_range();
        }
        match Directionality::of(app, context) {
            TextDirection::Rtl => format!("{end_text} \u{2013} {start_text}"),
            TextDirection::Ltr => format!("{start_text} \u{2013} {end_text}"),
        }
    }
}

impl StatelessWidget for InputDateRangePickerDialog {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let use_material3 = theme.use_material3;
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let orientation = MediaQuery::orientation_of(app, context);
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);

        // There's no M3 spec for a landscape layout input (not calendar) date range picker. To
        // ensure that the date range displayed in the input date range picker's header fits in
        // landscape mode, we override the M3 default here.
        let mut headline_style: Option<TextStyle> = if orientation == Orientation::Portrait {
            date_picker_theme
                .header_headline_style
                .clone()
                .or_else(|| defaults.header_headline_style(app))
        } else {
            theme.text_theme.headline_small.clone()
        };
        let header_foreground_color = date_picker_theme
            .header_foreground_color
            .or_else(|| defaults.header_foreground_color(app));
        headline_style = headline_style.map(|style| match header_foreground_color {
            Some(color) => style.copy_with().color(color),
            None => style,
        });

        let date_text = self.format_date_range(
            app,
            context,
            self.selected_start_date.as_ref(),
            self.selected_end_date.as_ref(),
            &self.current_date,
        );
        let semantic_date_text = match (self.selected_start_date, self.selected_end_date) {
            (Some(start), Some(end)) => format!(
                "{} \u{2013} {}",
                self.calendar_delegate
                    .format_medium_date(&start, &*localizations),
                self.calendar_delegate
                    .format_medium_date(&end, &*localizations)
            ),
            _ => String::new(),
        };

        let header = DatePickerHeader {
            help_text: self.help_text.clone(),
            title_text: date_text,
            title_semantics_label: Some(semantic_date_text),
            title_style: headline_style,
            orientation,
            is_short: orientation == Orientation::Landscape,
            entry_mode_button: self.entry_mode_button.clone(),
        }
        .into_widget();

        let actions = ConstrainedBox::new(BoxConstraints {
            min_height: 52.0,
            ..BoxConstraints::new()
        })
        .child(
            Padding::new(EdgeInsets::symmetric(0.0, 8.0).into()).child(
                Align::new()
                    .alignment(AlignmentGeometry::from(AlignmentDirectional::CENTER_END))
                    .child(OverflowBar::new().spacing(8.0).children(vec![
                        TextButton::new(
                            Some(self.on_cancel.clone()),
                            Some(Text::new(self.cancel_text.clone())),
                        )
                        .into_widget(),
                        TextButton::new(
                            Some(self.on_confirm.clone()),
                            Some(Text::new(self.confirm_text.clone())),
                        )
                        .into_widget(),
                    ])),
            ),
        )
        .into_widget();

        let text_scale_factor = MediaQuery::text_scaler_of(app, context)
            .clamp(0.0, MAX_RANGE_TEXT_SCALE_FACTOR)
            .scale(FONT_SIZE_TO_SCALE)
            / FONT_SIZE_TO_SCALE;
        let dialog_size = (if use_material3 {
            INPUT_PORTRAIT_DIALOG_SIZE_M3
        } else {
            INPUT_PORTRAIT_DIALOG_SIZE_M2
        }) * text_scale_factor;
        let picker = self.picker.clone();
        match orientation {
            Orientation::Portrait => LayoutBuilder::new(move |_app, _context, constraints| {
                let portrait_dialog_size = if use_material3 {
                    INPUT_PORTRAIT_DIALOG_SIZE_M3
                } else {
                    INPUT_PORTRAIT_DIALOG_SIZE_M2
                };
                // Make sure the portrait dialog can fit the contents comfortably when resized
                // from the landscape dialog.
                let is_fully_portrait = constraints.constraints().max_height
                    >= dialog_size.height().min(portrait_dialog_size.height());
                let mut children = Vec::new();
                // When the portrait dialog does not fit vertically, hide the header.
                if is_fully_portrait {
                    children.push(header.clone());
                }
                children.push(Expanded::new(picker.clone()).into_widget());
                children.push(actions.clone());
                waft_widgets::Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children(children)
                    .into_widget()
            })
            .into_widget(),
            Orientation::Landscape => Row::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children([
                    header,
                    Flexible::new(
                        waft_widgets::Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .children([Expanded::new(picker).into_widget(), actions]),
                    )
                    .into_widget(),
                ])
                .into_widget(),
        }
    }
}

/// Dart's `_InputDateRangePicker`.
#[derive(Clone)]
struct InputDateRangePicker {
    key: Option<waft_widgets::KeyRef>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
    initial_start_date: Option<DateTime>,
    initial_end_date: Option<DateTime>,
    first_date: DateTime,
    last_date: DateTime,
    selectable_day_predicate: Option<SelectableDayForRangePredicate>,
    on_start_date_changed: Option<ValueChanged<Option<DateTime>>>,
    on_end_date_changed: Option<ValueChanged<Option<DateTime>>>,
    autofocus: bool,
    autovalidate: bool,
    #[expect(dead_code, reason = "Dart's field is declared and never read either")]
    help_text: Option<String>,
    error_invalid_range_text: Option<String>,
    error_format_text: Option<String>,
    error_invalid_text: Option<String>,
    field_start_hint_text: Option<String>,
    field_end_hint_text: Option<String>,
    field_start_label_text: Option<String>,
    field_end_label_text: Option<String>,
    keyboard_type: TextInputType,
}

impl fmt::Debug for InputDateRangePicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputDateRangePicker")
            .field("initial_start_date", &self.initial_start_date)
            .field("initial_end_date", &self.initial_end_date)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for InputDateRangePicker {
    type State = InputDateRangePickerState;

    fn key(&self) -> Option<&waft_widgets::KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> InputDateRangePickerState {
        InputDateRangePickerState {
            state: StateData::new(),
            start_controller: None,
            end_controller: None,
            start_date: self
                .initial_start_date
                .map(|date| self.calendar_delegate.date_only(&date)),
            end_date: self
                .initial_end_date
                .map(|date| self.calendar_delegate.date_only(&date)),
            start_error_text: None,
            end_error_text: None,
            auto_selected: false,
        }
    }
}

/// Dart's `_InputDateRangePickerState`.
struct InputDateRangePickerState {
    state: StateData<InputDateRangePicker>,
    /// Created in `init_state`, because a state is created without the [`App`].
    start_controller: Option<Ref<TextEditingController>>,
    end_controller: Option<Ref<TextEditingController>>,
    start_date: Option<DateTime>,
    end_date: Option<DateTime>,
    start_error_text: Option<String>,
    end_error_text: Option<String>,
    auto_selected: bool,
}

impl InputDateRangePickerState {
    fn start_controller(self: Handle<Self>, app: &App) -> Ref<TextEditingController> {
        app.get(self)
            .start_controller
            .clone()
            .expect("created in init_state")
    }

    fn end_controller(self: Handle<Self>, app: &App) -> Ref<TextEditingController> {
        app.get(self)
            .end_controller
            .clone()
            .expect("created in init_state")
    }

    /// Dart's `validate`.
    fn validate(self: Handle<Self>, app: &mut App) -> bool {
        let mut start_error = self.validate_date(app, app.get(self).start_date);
        let end_error = self.validate_date(app, app.get(self).end_date);
        if start_error.is_none() && end_error.is_none() {
            let state = app.get(self);
            if state
                .start_date
                .expect("a valid start date")
                .is_after(&state.end_date.expect("a valid end date"))
            {
                start_error = Some(
                    self.widget(app)
                        .error_invalid_range_text
                        .clone()
                        .unwrap_or_else(|| {
                            let context = self.context(app);
                            <dyn MaterialLocalizations>::of(app, context).invalid_date_range_label()
                        }),
                );
            }
        }
        self.set_state(app, |_| {});
        let valid = start_error.is_none() && end_error.is_none();
        let state = app.get_mut(self);
        state.start_error_text = start_error;
        state.end_error_text = end_error;
        valid
    }

    fn parse_date(self: Handle<Self>, app: &mut App, text: &str) -> Option<DateTime> {
        let context = self.context(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        self.widget(app)
            .calendar_delegate
            .parse_compact_date(Some(text), &*localizations)
    }

    fn validate_date(self: Handle<Self>, app: &mut App, date: Option<DateTime>) -> Option<String> {
        let context = self.context(app);
        match date {
            None => Some(
                self.widget(app)
                    .error_format_text
                    .clone()
                    .unwrap_or_else(|| {
                        <dyn MaterialLocalizations>::of(app, context).invalid_date_format_label()
                    }),
            ),
            Some(date) if !self.is_day_selectable(app, date) => Some(
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

    fn is_day_selectable(self: Handle<Self>, app: &App, day: DateTime) -> bool {
        let widget = self.widget(app);
        if day.is_before(&widget.first_date) || day.is_after(&widget.last_date) {
            return false;
        }
        match &widget.selectable_day_predicate {
            None => true,
            Some(predicate) => {
                let state = app.get(self);
                predicate(day, state.start_date, state.end_date)
            }
        }
    }

    fn update_controller(
        self: Handle<Self>,
        app: &mut App,
        controller: Ref<TextEditingController>,
        text: &str,
        select_text: bool,
    ) {
        let mut value = controller
            .handle()
            .value(app)
            .clone()
            .text(text.to_string());
        if select_text {
            value = value.selection(TextSelection::new(0, text.encode_utf16().count() as i32));
        }
        controller.handle().set_value(app, value);
    }

    fn handle_start_changed(self: Handle<Self>, app: &mut App, text: String) {
        let date = self.parse_date(app, &text);
        self.set_state(app, |_| {});
        app.get_mut(self).start_date = date;
        if let Some(on_start_date_changed) = self.widget(app).on_start_date_changed.clone() {
            on_start_date_changed(app, date);
        }
        if self.widget(app).autovalidate {
            self.validate(app);
        }
    }

    fn handle_end_changed(self: Handle<Self>, app: &mut App, text: String) {
        let date = self.parse_date(app, &text);
        self.set_state(app, |_| {});
        app.get_mut(self).end_date = date;
        if let Some(on_end_date_changed) = self.widget(app).on_end_date_changed.clone() {
            on_end_date_changed(app, date);
        }
        if self.widget(app).autovalidate {
            self.validate(app);
        }
    }
}

impl fmt::Debug for InputDateRangePickerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputDateRangePickerState")
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .finish_non_exhaustive()
    }
}

impl State for InputDateRangePickerState {
    type Widget = InputDateRangePicker;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let start_controller = TextEditingController::new(app);
        let end_controller = TextEditingController::new(app);
        let state = app.get_mut(self);
        state.start_controller = Some(start_controller);
        state.end_controller = Some(end_controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.start_controller(app).handle().dispose(app);
        self.end_controller(app).handle().dispose(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let (calendar_delegate, autofocus) = {
            let widget = self.widget(app);
            (Rc::clone(&widget.calendar_delegate), widget.autofocus)
        };
        if let Some(start_date) = app.get(self).start_date {
            let text = calendar_delegate.format_compact_date(&start_date, &*localizations);
            let select_text = autofocus && !app.get(self).auto_selected;
            let controller = self.start_controller(app);
            self.update_controller(app, controller, &text, select_text);
            app.get_mut(self).auto_selected = select_text;
        }
        if let Some(end_date) = app.get(self).end_date {
            let text = calendar_delegate.format_compact_date(&end_date, &*localizations);
            let controller = self.end_controller(app);
            self.update_controller(app, controller, &text, false);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let use_material3 = theme.use_material3;
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let input_theme = InputDecorationTheme::of(app, context);
        let input_border = input_theme.border.clone().unwrap_or_else(|| {
            if use_material3 {
                InputBorderRef::new(OutlineInputBorder::new())
            } else {
                InputBorderRef::new(UnderlineInputBorder::new())
            }
        });
        let widget = self.widget(app).clone();
        let start_error_text = app.get(self).start_error_text.clone();
        let end_error_text = app.get(self).end_error_text.clone();
        let input_formatters = widget
            .calendar_delegate
            .keyboard_input_formatters(&*localizations);

        let mut start_decoration = InputDecoration::new()
            .border(input_border.clone())
            .filled(input_theme.filled)
            .hint_text(
                widget
                    .field_start_hint_text
                    .clone()
                    .unwrap_or_else(|| widget.calendar_delegate.date_help_text(&*localizations)),
            )
            .label_text(
                widget
                    .field_start_label_text
                    .clone()
                    .unwrap_or_else(|| localizations.date_range_start_label()),
            );
        if let Some(error_text) = start_error_text {
            start_decoration = start_decoration.error_text(error_text);
        }
        let mut end_decoration = InputDecoration::new()
            .border(input_border)
            .filled(input_theme.filled)
            .hint_text(
                widget
                    .field_end_hint_text
                    .clone()
                    .unwrap_or_else(|| widget.calendar_delegate.date_help_text(&*localizations)),
            )
            .label_text(
                widget
                    .field_end_label_text
                    .clone()
                    .unwrap_or_else(|| localizations.date_range_end_label()),
            );
        if let Some(error_text) = end_error_text {
            end_decoration = end_decoration.error_text(error_text);
        }

        let mut start_field = TextField::new()
            .controller(self.start_controller(app).erase())
            .decoration(Some(start_decoration))
            .keyboard_type(widget.keyboard_type)
            .on_changed(move |app, text| self.handle_start_changed(app, text))
            .autofocus(widget.autofocus);
        let mut end_field = TextField::new()
            .controller(self.end_controller(app).erase())
            .decoration(Some(end_decoration))
            .keyboard_type(widget.keyboard_type)
            .on_changed(move |app, text| self.handle_end_changed(app, text));
        if let Some(input_formatters) = input_formatters {
            start_field = start_field.input_formatters(input_formatters.clone());
            end_field = end_field.input_formatters(input_formatters);
        }

        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children([
                Expanded::new(start_field).into_widget(),
                SizedBox::new().width(8.0).into_widget(),
                Expanded::new(end_field).into_widget(),
            ])
            .into_widget()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! date_picker_test.dart and date_range_picker_test.dart: the dialogs a user sees, picks a date
    //! or a range in and confirms or cancels.

    use waft_widgets::testing::{
        TestPointer, WidgetTester, expect, find, finds_n_widgets, finds_nothing, finds_one_widget,
        test_widgets,
    };

    use super::*;
    use crate::date::{DateTimeRange, GregorianCalendarDelegate};
    use crate::dialog::Dialog;
    use crate::elevated_button::ElevatedButton;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::test_support::material_app;
    use crate::text_field::TextField;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3};
    use std::cell::RefCell;
    use std::rc::Rc;
    use waft_embedder::{
        Offset, Size, TargetPlatform, TextAffinity, TextEditingValue, TextSelection,
    };
    use waft_foundation::{DateTime, Listener};
    use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder, ShapeBorderRef};
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey, TextInputClient};
    use waft_widgets::{
        AnyElement, Builder, Center, EditableText, EditableTextState, IntoWidget, Text,
    };

    /// Dart's `tester.widget<TextField>(find.byType(TextField)).controller!.text`: the text of
    /// the first `TextField`'s controller.
    pub(crate) fn text_field_controller_text(tester: &WidgetTester) -> String {
        let field = tester.element(find::by_type::<TextField>().at(0));
        let controller = tester.widget(field, |field: &TextField| {
            field.controller.clone().expect("a controller")
        });
        controller.text_value(&tester.cell.borrow()).to_owned()
    }

    pub(super) type PickerResult = Rc<RefCell<Option<CompleterFuture<RouteResult>>>>;

    /// A tester whose view is `size` physical pixels at a device pixel ratio of 1: Dart's
    /// `tester.view.physicalSize = size; tester.view.devicePixelRatio = 1.0`.
    pub(super) fn tester_of_size(size: Size) -> WidgetTester {
        WidgetTester::with_view(
            waft_test::TestPlatform::new().on(TargetPlatform::Android),
            waft_test::TestView::with_pixel_ratio(size.width(), size.height(), 1.0),
        )
    }

    /// What the host does when the view's size changes: Dart's `tester.view.physicalSize = size`
    /// followed by the binding's metrics change. The tester's views here are at a device pixel
    /// ratio of 1 (see [`tester_of_size`]).
    pub(super) fn resize_view(tester: &mut WidgetTester, size: Size) {
        tester.view().resize(size.width(), size.height());
        tester.update(|app| {
            waft_rendering::RendererBinding::instance(app).handle_metrics_changed(app);
        });
    }

    /// Runs a case whose Dart source sets the view size first: `test_widgets` for its stack,
    /// with the sized tester inside.
    pub(super) fn test_widgets_sized(size: Size, body: impl FnOnce(&mut WidgetTester) + Send) {
        test_widgets(move |_| {
            let mut tester = tester_of_size(size);
            body(&mut tester);
        });
    }

    /// Dart's `wideWindowSize` and `narrowWindowSize`, set as the physical size at a device pixel
    /// ratio of 1, so the logical size here.
    pub(super) const WIDE_WINDOW_SIZE: Size = Size::new(1920.0, 1080.0);
    pub(super) const NARROW_WINDOW_SIZE: Size = Size::new(1070.0, 1770.0);

    /// The variables of date_picker_test.dart's `setUp`, which each case may change.
    #[derive(Clone)]
    pub(super) struct DatePickerOptions {
        pub(super) first_date: DateTime,
        pub(super) last_date: DateTime,
        pub(super) initial_date: Option<DateTime>,
        pub(super) today: DateTime,
        pub(super) selectable_day_predicate: Option<waft_widgets::SelectableDayPredicate>,
        pub(super) initial_entry_mode: DatePickerEntryMode,
        pub(super) initial_calendar_mode: DatePickerMode,
        pub(super) cancel_text: Option<String>,
        pub(super) confirm_text: Option<String>,
        pub(super) error_format_text: Option<String>,
        pub(super) error_invalid_text: Option<String>,
        pub(super) field_hint_text: Option<String>,
        pub(super) field_label_text: Option<String>,
        pub(super) help_text: Option<String>,
        pub(super) use_material3: bool,
        pub(super) current_mode: Rc<RefCell<DatePickerEntryMode>>,
        pub(super) builder: Option<waft_widgets::TransitionBuilder>,
        pub(super) keyboard_type: Option<waft_services::TextInputType>,
        pub(super) day_builder: Option<crate::calendar_date_picker::CalendarDatePickerDayBuilder>,
        pub(super) weekday_builder:
            Option<crate::calendar_date_picker::CalendarDatePickerWeekdayBuilder>,
        pub(super) calendar_delegate: Rc<dyn CalendarDelegate>,
    }

    impl DatePickerOptions {
        pub(super) fn new() -> DatePickerOptions {
            DatePickerOptions {
                first_date: DateTime::utc(2001, 1, 1),
                last_date: DateTime::utc(2031, 12, 31),
                initial_date: Some(DateTime::utc(2016, 1, 15)),
                today: DateTime::utc(2016, 1, 3),
                selectable_day_predicate: None,
                initial_entry_mode: DatePickerEntryMode::Calendar,
                initial_calendar_mode: DatePickerMode::Day,
                cancel_text: None,
                confirm_text: None,
                error_format_text: None,
                error_invalid_text: None,
                field_hint_text: None,
                field_label_text: None,
                help_text: None,
                use_material3: false,
                current_mode: Rc::new(RefCell::new(DatePickerEntryMode::Calendar)),
                builder: None,
                keyboard_type: None,
                day_builder: None,
                weekday_builder: None,
                calendar_delegate: Rc::new(GregorianCalendarDelegate),
            }
        }
    }

    /// Dart's `prepareDatePicker`: an app with a button, tapped, that shows the date picker.
    pub(super) fn prepare_date_picker(
        tester: &mut WidgetTester,
        options: &DatePickerOptions,
    ) -> PickerResult {
        let theme: ThemeData = if options.use_material3 {
            m3(&tester.cell.borrow())
        } else {
            m2(&tester.cell.borrow())
        };
        let result: PickerResult = Rc::default();
        let launcher = {
            let (options, result) = (options.clone(), Rc::clone(&result));
            Builder::new(move |_app, _context| {
                let (options, result) = (options.clone(), Rc::clone(&result));
                Material::new()
                    .child(Center::new().child(Builder::new(move |_app, context| {
                        let (options, result) = (options.clone(), Rc::clone(&result));
                        ElevatedButton::new(
                            Some(Listener::new(move |app| {
                                let current_mode = Rc::clone(&options.current_mode);
                                let future = show_date_picker(
                                    app,
                                    context,
                                    options.initial_date,
                                    options.first_date,
                                    options.last_date,
                                    Some(options.today),
                                    options.initial_entry_mode,
                                    options.selectable_day_predicate.clone(),
                                    options.help_text.clone(),
                                    options.cancel_text.clone(),
                                    options.confirm_text.clone(),
                                    None,
                                    true,
                                    None,
                                    None,
                                    true,
                                    None,
                                    None,
                                    options.builder.clone(),
                                    options.initial_calendar_mode,
                                    options.error_format_text.clone(),
                                    options.error_invalid_text.clone(),
                                    options.field_hint_text.clone(),
                                    options.field_label_text.clone(),
                                    options.keyboard_type,
                                    Some(Rc::new(move |_app, value| {
                                        *current_mode.borrow_mut() = value;
                                    })),
                                    None,
                                    None,
                                    options.day_builder.clone(),
                                    options.weekday_builder.clone(),
                                    Rc::clone(&options.calendar_delegate),
                                );
                                *result.borrow_mut() = Some(future);
                            })),
                            Some(Text::new("Go")),
                        )
                        .into_widget()
                    })))
                    .into_widget()
            })
        };
        tester.pump_widget(material_app(theme, launcher.into_widget()));
        tester.tap(find::text("Go"));
        tester.pump_and_settle();
        result
    }

    /// Dart's `await tester.tap(find.byIcon(icon)); await tester.pumpAndSettle();`.
    pub(super) fn tap_icon(tester: &mut WidgetTester, icon: &waft_widgets::IconData) {
        tester.tap(find::by_icon(icon));
        tester.pump_and_settle();
    }

    pub(super) fn text_field_count(tester: &WidgetTester) -> usize {
        find::by_type::<TextField>().evaluate(tester).len()
    }

    /// What the date picker completed with.
    pub(super) fn picked_date(result: &PickerResult) -> Option<DateTime> {
        result
            .borrow()
            .as_ref()
            .and_then(|future| future.peek())
            .flatten()
            .and_then(|value| value.downcast_ref::<DateTime>().copied())
    }

    pub(super) fn picked_range(result: &PickerResult) -> Option<DateTimeRange> {
        result
            .borrow()
            .as_ref()
            .and_then(|future| future.peek())
            .flatten()
            .and_then(|value| value.downcast_ref::<DateTimeRange>().copied())
    }

    pub(super) fn is_completed(result: &PickerResult) -> bool {
        result
            .borrow()
            .as_ref()
            .is_some_and(|future| future.peek().is_some())
    }

    pub(super) fn enter_text(tester: &mut WidgetTester, text: &str) {
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

    /// Dart's `find.text(text), findsOneWidget`.
    pub(super) fn has_text(tester: &WidgetTester, text: &str) -> bool {
        find::text(text).evaluate(tester).len() == 1
    }

    /// Dart's `find.text(text), findsNothing`.
    pub(super) fn has_no_text(tester: &WidgetTester, text: &str) -> bool {
        find::text(text).evaluate(tester).is_empty()
    }

    pub(super) fn dialog_container_size(tester: &WidgetTester) -> Size {
        let container = tester.element(find::by_type::<waft_widgets::AnimatedContainer>().at(0));
        tester.get_size(container)
    }

    #[test]
    fn material3_default_dialog_size() {
        test_widgets(|_| {
            let mut options = DatePickerOptions::new();
            options.use_material3 = true;

            // Test landscape layout.
            let tester = &mut tester_of_size(WIDE_WINDOW_SIZE);
            prepare_date_picker(tester, &options);
            assert_eq!(
                dialog_container_size(tester),
                CALENDAR_LANDSCAPE_DIALOG_SIZE
            );

            // Test portrait layout.
            let tester = &mut tester_of_size(NARROW_WINDOW_SIZE);
            prepare_date_picker(tester, &options);
            assert_eq!(
                dialog_container_size(tester),
                CALENDAR_PORTRAIT_DIALOG_SIZE_M3
            );
        });
    }

    #[test]
    fn material3_uses_sentence_case_labels() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.use_material3 = true;
            prepare_date_picker(tester, &options);
            assert!(has_text(tester, "Select date"));
        });
    }

    #[test]
    fn cancel_confirm_and_help_text_is_used() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.cancel_text = Some("nope".to_string());
            options.confirm_text = Some("yep".to_string());
            options.help_text = Some("help".to_string());
            prepare_date_picker(tester, &options);
            assert!(has_text(tester, "nope"));
            assert!(has_text(tester, "yep"));
            assert!(has_text(tester, "help"));
        });
    }

    #[test]
    fn initial_date_is_the_default() {
        test_widgets(|tester| {
            let result = prepare_date_picker(tester, &DatePickerOptions::new());
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 1, 15)));
        });
    }

    #[test]
    fn can_cancel() {
        test_widgets(|tester| {
            let result = prepare_date_picker(tester, &DatePickerOptions::new());
            tester.tap(find::text("CANCEL"));
            tester.pump_and_settle();
            assert!(is_completed(&result));
            assert_eq!(picked_date(&result), None);
        });
    }

    #[test]
    fn can_switch_from_calendar_to_input_entry_mode() {
        test_widgets(|tester| {
            prepare_date_picker(tester, &DatePickerOptions::new());
            assert_eq!(text_field_count(tester), 0);
            tap_icon(tester, &Icons::edit());
            assert_eq!(text_field_count(tester), 1);
        });
    }

    #[test]
    fn can_switch_from_input_to_calendar_entry_mode() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            prepare_date_picker(tester, &options);
            assert_eq!(text_field_count(tester), 1);
            tap_icon(tester, &Icons::calendar_today());
            assert_eq!(text_field_count(tester), 0);
        });
    }

    #[test]
    fn can_not_switch_out_of_calendar_only_mode() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::CalendarOnly;
            prepare_date_picker(tester, &options);
            assert_eq!(text_field_count(tester), 0);
            assert!(find::by_icon(&Icons::edit()).evaluate(tester).is_empty());
        });
    }

    #[test]
    fn can_not_switch_out_of_input_only_mode() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::InputOnly;
            prepare_date_picker(tester, &options);
            assert_eq!(text_field_count(tester), 1);
            assert!(
                find::by_icon(&Icons::calendar_today())
                    .evaluate(tester)
                    .is_empty()
            );
        });
    }

    #[test]
    fn switching_to_input_mode_keeps_selected_date() {
        test_widgets(|tester| {
            let result = prepare_date_picker(tester, &DatePickerOptions::new());
            tester.tap(find::text("12"));
            tap_icon(tester, &Icons::edit());
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 1, 12)));
        });
    }

    #[test]
    fn callback_on_toggle_of_date_picker_mode() {
        test_widgets(|tester| {
            let options = DatePickerOptions::new();
            prepare_date_picker(tester, &options);
            tap_icon(tester, &Icons::edit());
            assert_eq!(*options.current_mode.borrow(), DatePickerEntryMode::Input);
            assert_eq!(text_field_count(tester), 1);
            tap_icon(tester, &Icons::calendar_today());
            assert_eq!(
                *options.current_mode.borrow(),
                DatePickerEntryMode::Calendar
            );
            assert_eq!(text_field_count(tester), 0);
        });
    }

    #[test]
    fn can_select_a_day() {
        test_widgets(|tester| {
            let result = prepare_date_picker(tester, &DatePickerOptions::new());
            tester.tap(find::text("12"));
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 1, 12)));
        });
    }

    #[test]
    fn can_select_a_day_with_no_initial_date() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_date = None;
            let result = prepare_date_picker(tester, &options);
            tester.tap(find::text("12"));
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 1, 12)));
        });
    }

    #[test]
    fn only_predicate_days_are_selectable() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_date = Some(DateTime::utc(2017, 1, 16));
            options.first_date = DateTime::utc(2017, 1, 10);
            options.last_date = DateTime::utc(2017, 1, 20);
            options.selectable_day_predicate = Some(Rc::new(|day| day.day() % 2 == 0));
            let result = prepare_date_picker(tester, &options);
            tester.tap(find::text("13")); // Odd, doesn't work.
            tester.tap(find::text("10")); // Even, works.
            tester.tap(find::text("17")); // Odd, doesn't work.
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2017, 1, 10)));
        });
    }

    #[test]
    fn input_mode_initial_entry_mode_is_used() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            prepare_date_picker(tester, &options);
            assert_eq!(text_field_count(tester), 1);
        });
    }

    #[test]
    fn input_mode_hint_label_and_help_text_is_used() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            options.cancel_text = Some("nope".to_string());
            options.confirm_text = Some("yep".to_string());
            options.field_hint_text = Some("hint".to_string());
            options.field_label_text = Some("label".to_string());
            options.help_text = Some("help".to_string());
            prepare_date_picker(tester, &options);
            assert!(has_text(tester, "nope"));
            assert!(has_text(tester, "yep"));
            assert!(has_text(tester, "hint"));
            assert!(has_text(tester, "label"));
            assert!(has_text(tester, "help"));
        });
    }

    #[test]
    fn input_mode_initial_date_is_the_default() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            let result = prepare_date_picker(tester, &options);
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 1, 15)));
        });
    }

    #[test]
    fn input_mode_toggle_to_calendar_mode_keeps_selected_date() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            let result = prepare_date_picker(tester, &options);
            enter_text(tester, "12/25/2016");
            tap_icon(tester, &Icons::calendar_today());
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 12, 25)));
        });
    }

    #[test]
    fn input_mode_entered_text_returns_date() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            let result = prepare_date_picker(tester, &options);
            enter_text(tester, "12/25/2016");
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 12, 25)));
        });
    }

    #[test]
    fn input_mode_too_short_entered_text_shows_error() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            options.error_format_text = Some("oops".to_string());
            prepare_date_picker(tester, &options);
            tester.pump_and_settle();
            enter_text(tester, "1225");
            assert!(has_no_text(tester, "oops"));

            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert!(has_text(tester, "oops"));
        });
    }

    #[test]
    fn input_mode_bad_format_entered_text_shows_error() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            options.error_format_text = Some("oops".to_string());
            prepare_date_picker(tester, &options);
            tester.pump_and_settle();
            enter_text(tester, "20 days, 3 months, 2003");
            // Dart's `find.text` finds the `EditableText` showing it; `find_text` reads `Text` only.
            assert_eq!(
                text_field_controller_text(tester),
                "20 days, 3 months, 2003"
            );
            assert!(has_no_text(tester, "oops"));

            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert!(has_text(tester, "oops"));
        });
    }

    #[test]
    fn input_mode_invalid_entered_text_shows_error() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::Input;
            options.error_invalid_text = Some("oops".to_string());
            prepare_date_picker(tester, &options);
            tester.pump_and_settle();
            enter_text(tester, "08/10/1969");
            assert!(has_no_text(tester, "oops"));

            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert!(has_text(tester, "oops"));
        });
    }

    #[test]
    fn input_only_mode_should_validate_date() {
        test_widgets(|tester| {
            let mut options = DatePickerOptions::new();
            options.initial_entry_mode = DatePickerEntryMode::InputOnly;
            prepare_date_picker(tester, &options);
            // Enter text input mode and type an invalid date to get error.
            enter_text(tester, "1234567");
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert!(has_text(tester, "Invalid format."));
        });
    }

    // The Material 2 'Default dialog size'.
    #[test]
    fn material2_default_dialog_size() {
        test_widgets(|_| {
            let options = DatePickerOptions::new();

            // Test landscape layout.
            let tester = &mut tester_of_size(WIDE_WINDOW_SIZE);
            prepare_date_picker(tester, &options);
            assert_eq!(
                dialog_container_size(tester),
                CALENDAR_LANDSCAPE_DIALOG_SIZE
            );

            // Test portrait layout.
            let tester = &mut tester_of_size(NARROW_WINDOW_SIZE);
            prepare_date_picker(tester, &options);
            assert_eq!(
                dialog_container_size(tester),
                CALENDAR_PORTRAIT_DIALOG_SIZE_M2
            );
        });
    }

    pub(super) fn range(start: (i32, i32, i32), end: (i32, i32, i32)) -> DateTimeRange {
        DateTimeRange::new(
            DateTime::utc(start.0, start.1, start.2),
            DateTime::utc(end.0, end.1, end.2),
        )
    }

    /// Enters `text` in the `index`th text field of the input range picker.
    pub(super) fn enter_text_in_field(tester: &mut WidgetTester, index: usize, text: &str) {
        let editable = tester.element(find::by_type::<EditableText>().at(index));
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

    #[derive(Clone, Copy)]
    pub(super) enum Arrow {
        Left,
        Right,
        Up,
        Down,
    }

    pub(super) fn dialog_material(tester: &WidgetTester) -> AnyElement {
        tester.element(
            find::descendant(find::by_type::<Dialog>().at(0), find::by_type::<Material>()).first(),
        )
    }

    pub(super) fn rounded(radius: f64) -> ShapeBorderRef {
        ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::circular(radius),
        ))
    }

    pub(super) fn top_left(tester: &WidgetTester, element: AnyElement) -> Offset {
        tester.get_rect(element).top_left()
    }

    pub(super) fn bottom_left(tester: &WidgetTester, element: AnyElement) -> Offset {
        let rect = tester.get_rect(element);
        Offset::new(rect.left, rect.bottom)
    }

    pub(super) fn bottom_right(tester: &WidgetTester, element: AnyElement) -> Offset {
        let rect = tester.get_rect(element);
        Offset::new(rect.right, rect.bottom)
    }

    pub(super) fn header_font_size(tester: &WidgetTester, text: &str) -> Option<f64> {
        tester.widget(tester.element(find::text(text)), |text: &Text| {
            text.style.as_ref().and_then(|style| style.font_size)
        })
    }

    pub(super) fn press(
        tester: &mut WidgetTester,
        physical: PhysicalKeyboardKey,
        logical: LogicalKeyboardKey,
    ) {
        tester.send_key_event(physical, logical);
    }

    pub(super) fn tab(tester: &mut WidgetTester) {
        press(tester, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
    }

    pub(super) fn tabs(tester: &mut WidgetTester, count: usize) {
        for _ in 0..count {
            tab(tester);
        }
    }

    pub(super) fn space(tester: &mut WidgetTester) {
        press(
            tester,
            PhysicalKeyboardKey::SPACE,
            LogicalKeyboardKey::SPACE,
        );
    }

    pub(super) fn arrows(tester: &mut WidgetTester, keys: &[Arrow]) {
        for key in keys {
            let (physical, logical) = match key {
                Arrow::Left => (
                    PhysicalKeyboardKey::ARROW_LEFT,
                    LogicalKeyboardKey::ARROW_LEFT,
                ),
                Arrow::Right => (
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT,
                ),
                Arrow::Up => (PhysicalKeyboardKey::ARROW_UP, LogicalKeyboardKey::ARROW_UP),
                Arrow::Down => (
                    PhysicalKeyboardKey::ARROW_DOWN,
                    LogicalKeyboardKey::ARROW_DOWN,
                ),
            };
            press(tester, physical, logical);
        }
    }

    pub(super) fn icon_button_of(
        tester: &WidgetTester,
        icon: &waft_widgets::IconData,
    ) -> AnyElement {
        tester.element(
            find::ancestor(
                find::by_icon(icon).first(),
                find::by_type::<crate::icon_button::IconButton>(),
            )
            .first(),
        )
    }

    mod dialog {
        //! date_picker_test.dart: the dialog's properties, its routes and barrier, its layout, the
        //! selection of days, months and years, the keyboard navigation and the calendar delegates.

        use waft_widgets::testing::assert_f32_equals;

        use std::cell::Cell;
        use std::rc::Rc;

        use waft_embedder::{Offset, Size};
        use waft_foundation::{App, DateTime, Handle, Listener};
        use waft_painting::{
            BorderRadiusGeometry, BorderSide, EdgeInsets, RoundedRectangleBorder, TextDirection,
        };
        use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey, TextInputType};
        use waft_widgets::{
            AnyElement, BuildContext, Builder, Center, Icon, IntoWidget, ModalBarrier, SizedBox,
            Text, TransitionBuilder,
        };

        use super::*;
        use super::{
            DatePickerOptions, enter_text, has_no_text, has_text, picked_date, prepare_date_picker,
            resize_view, tap_icon, tester_of_size, text_field_count,
        };
        use crate::colors::Colors;
        use crate::date::GregorianCalendarDelegate;
        use crate::dialog::Dialog;
        use crate::elevated_button::ElevatedButton;
        use crate::icons::Icons;
        use crate::material::Material;
        use crate::material_localizations::MaterialLocalizations;
        use crate::test_support::{PaintInspection, material_app};

        use crate::text_field::TextField;
        use crate::theme_data::ThemeData;
        use crate::tooltip::tests::{m2, m3};
        use waft_rendering::testing::{PaintCall, assert_paints, paints};

        /// The arguments of the `showDatePicker` calls that are not `prepareDatePicker`'s.
        #[derive(Clone)]
        struct Show {
            initial_entry_mode: DatePickerEntryMode,
            builder: Option<TransitionBuilder>,
            barrier_dismissible: bool,
            barrier_color: Option<waft_embedder::Color>,
            barrier_label: Option<String>,
            use_root_navigator: bool,
            switch_to_input_entry_mode_icon: Option<fn() -> Icon>,
            switch_to_calendar_entry_mode_icon: Option<fn() -> Icon>,
            calendar_delegate: Rc<dyn CalendarDelegate>,
            /// `(initialDate, firstDate, lastDate)`, when not `DateTime.now()`, 2018 and 2030.
            dates: Option<(DateTime, DateTime, DateTime)>,
            /// Where the future of the shown picker goes.
            result: Option<super::PickerResult>,
        }

        impl Show {
            fn new() -> Show {
                Show {
                    initial_entry_mode: DatePickerEntryMode::Calendar,
                    builder: None,
                    barrier_dismissible: true,
                    barrier_color: None,
                    barrier_label: None,
                    use_root_navigator: true,
                    switch_to_input_entry_mode_icon: None,
                    switch_to_calendar_entry_mode_icon: None,
                    calendar_delegate: Rc::new(GregorianCalendarDelegate),
                    dates: None,
                    result: None,
                }
            }

            /// `showDatePicker(initialDate: DateTime.now(), firstDate: DateTime(2018),
            /// lastDate: DateTime(2030), ...)`.
            fn show(&self, app: &mut App, context: BuildContext) -> CompleterFuture<RouteResult> {
                let (initial_date, first_date, last_date) = self.dates.unwrap_or((
                    DateTime::now(),
                    DateTime::utc(2018, 1, 1),
                    DateTime::utc(2030, 1, 1),
                ));
                let future = show_date_picker(
                    app,
                    context,
                    Some(initial_date),
                    first_date,
                    last_date,
                    None,
                    self.initial_entry_mode,
                    None,
                    None,
                    None,
                    None,
                    None,
                    self.barrier_dismissible,
                    self.barrier_color,
                    self.barrier_label.clone(),
                    self.use_root_navigator,
                    None,
                    None,
                    self.builder.clone(),
                    DatePickerMode::Day,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    self.switch_to_input_entry_mode_icon.map(|icon| icon()),
                    self.switch_to_calendar_entry_mode_icon.map(|icon| icon()),
                    None,
                    None,
                    Rc::clone(&self.calendar_delegate),
                );
                if let Some(result) = &self.result {
                    *result.borrow_mut() = Some(future.clone());
                }
                future
            }
        }

        /// An app with a button labelled `label` that shows a date picker.
        fn open_app(tester: &mut WidgetTester, material3: bool, label: &'static str, show: Show) {
            let theme = if material3 {
                m3(&tester.cell.borrow())
            } else {
                m2(&tester.cell.borrow())
            };
            open_app_with_theme(tester, theme, label, show);
        }

        fn open_app_with_theme(
            tester: &mut WidgetTester,
            theme: ThemeData,
            label: &'static str,
            show: Show,
        ) {
            let home = Material::new()
                .child(Center::new().child(Builder::new(move |_app, context| {
                    let show = show.clone();
                    ElevatedButton::new(
                        Some(Listener::new(move |app| drop(show.show(app, context)))),
                        Some(Text::new(label)),
                    )
                    .into_widget()
                })))
                .into_widget();
            tester.pump_widget(material_app(theme, home));
        }

        fn sized_box_builder() -> TransitionBuilder {
            Rc::new(|_app, _context, _child| SizedBox::shrink().into_widget())
        }

        fn directionality_builder(text_direction: TextDirection) -> TransitionBuilder {
            Rc::new(move |_app, _context, child| {
                waft_widgets::Directionality::new(
                    text_direction,
                    child.expect("a dialog child").clone(),
                )
                .into_widget()
            })
        }

        /// Dart's `_DatePickerObserver`: how many dialog routes the navigator shows.
        #[derive(Default)]
        struct DatePickerObserver {
            navigator_observer: waft_widgets::NavigatorObserverData,
            date_picker_count: Cell<i32>,
        }

        impl waft_widgets::NavigatorObserver for DatePickerObserver {
            waft_widgets::navigator_observer_accessors!();

            fn did_push(
                self: Handle<Self>,
                app: &mut App,
                route: waft_widgets::AnyRoute,
                _previous: Option<waft_widgets::AnyRoute>,
            ) {
                if route.downcast::<crate::dialog::DialogRoute>(app).is_some() {
                    let count = &app.get(self).date_picker_count;
                    count.set(count.get() + 1);
                }
            }

            fn did_pop(
                self: Handle<Self>,
                app: &mut App,
                route: waft_widgets::AnyRoute,
                _previous: Option<waft_widgets::AnyRoute>,
            ) {
                if route.downcast::<crate::dialog::DialogRoute>(app).is_some() {
                    let count = &app.get(self).date_picker_count;
                    count.set(count.get() - 1);
                }
            }
        }

        #[test]
        fn forwards_day_builder_and_weekday_builder_to_the_calendar() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.day_builder = Some(Rc::new(|_app, _context, details| {
                    if details.day.day() == 1 {
                        Text::new("Custom day 1").into_widget()
                    } else {
                        details.child
                    }
                }));
                options.weekday_builder = Some(Rc::new(|_app, _context, details| {
                    if details.weekday == DateTime::SUNDAY {
                        Text::new("Custom Sunday").into_widget()
                    } else {
                        details.child
                    }
                }));
                prepare_date_picker(tester, &options);
                expect(tester, &find::text("Custom day 1"), finds_one_widget());
                expect(tester, &find::text("Custom Sunday"), finds_one_widget());
            });
        }

        #[test]
        fn material3_default_dialog_properties() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.use_material3 = true;
                prepare_date_picker(tester, &options);
                let theme = m3(&tester.cell.borrow());

                let (color, shadow_color, surface_tint_color, elevation, shape, clip_behavior) =
                    tester.widget(dialog_material(tester), |material: &Material| {
                        (
                            material.color,
                            material.shadow_color,
                            material.surface_tint_color,
                            material.elevation,
                            material.shape.clone(),
                            material.clip_behavior,
                        )
                    });
                assert_eq!(color, Some(theme.color_scheme.surface_container_high()));
                assert_eq!(shadow_color, Some(Colors::TRANSPARENT));
                assert_eq!(surface_tint_color, Some(Colors::TRANSPARENT));
                assert_eq!(elevation, 6.0);
                assert_eq!(shape, Some(rounded(28.0)));
                assert_eq!(clip_behavior, waft_embedder::Clip::AntiAlias);

                let dialog = tester.element(find::by_type::<Dialog>().at(0));
                let inset_padding = tester.widget(dialog, |dialog: &Dialog| dialog.inset_padding);
                assert_eq!(inset_padding, Some(EdgeInsets::symmetric(24.0, 16.0)));
            });
        }

        #[test]
        fn material2_default_dialog_properties() {
            test_widgets(|tester| {
                prepare_date_picker(tester, &DatePickerOptions::new());
                let theme = m2(&tester.cell.borrow());

                let (color, shadow_color, elevation, shape, clip_behavior) =
                    tester.widget(dialog_material(tester), |material: &Material| {
                        (
                            material.color,
                            material.shadow_color,
                            material.elevation,
                            material.shape.clone(),
                            material.clip_behavior,
                        )
                    });
                assert_eq!(color, Some(theme.color_scheme.surface));
                assert_eq!(shadow_color, Some(theme.shadow_color));
                assert_eq!(elevation, 24.0);
                assert_eq!(shape, Some(rounded(4.0)));
                assert_eq!(clip_behavior, waft_embedder::Clip::AntiAlias);

                let dialog = tester.element(find::by_type::<Dialog>().at(0));
                let inset_padding = tester.widget(dialog, |dialog: &Dialog| dialog.inset_padding);
                assert_eq!(inset_padding, Some(EdgeInsets::symmetric(24.0, 16.0)));
            });
        }

        #[test]
        fn switching_to_input_mode_resets_input_error_state() {
            test_widgets(|tester| {
                prepare_date_picker(tester, &DatePickerOptions::new());

                // Enter text input mode and type an invalid date to get error.
                tap_icon(tester, &Icons::edit());
                enter_text(tester, "1234567");
                tester.tap(find::text("OK"));
                tester.pump_and_settle();
                assert!(has_text(tester, "Invalid format."));

                // Toggle to calendar mode and then back to input mode
                tap_icon(tester, &Icons::calendar_today());
                tap_icon(tester, &Icons::edit());
                assert!(has_no_text(tester, "Invalid format."));

                // Edit the text, the error should not be showing until ok is tapped
                enter_text(tester, "1234567");
                tester.pump_and_settle();
                assert!(has_no_text(tester, "Invalid format."));
            });
        }

        #[test]
        fn builder_parameter() {
            test_widgets(|tester| {
                let build_frame = |tester: &mut WidgetTester, text_direction: TextDirection| {
                    let mut show = Show::new();
                    show.builder = Some(directionality_builder(text_direction));
                    open_app(tester, true, "X", show);
                };

                build_frame(tester, TextDirection::Ltr);
                tester.tap(find::text("X"));
                tester.pump_and_settle();
                let ltr_ok_right = tester.get_rect(find::text("OK")).right;

                tester.tap(find::text("OK")); // Dismiss the dialog.
                tester.pump_and_settle();

                build_frame(tester, TextDirection::Rtl);
                tester.tap(find::text("X"));
                tester.pump_and_settle();

                // Verify that the time picker is being laid out RTL. We expect the left edge of the
                // 'OK' button in the RTL layout to match the gap between right edge of the 'OK'
                // button and the right edge of the 800 wide view.
                let rtl_ok_left = tester.get_rect(find::text("OK")).left;
                // Dart's `moreOrLessEquals`, at the f32 precision Valo lays out in.
                let expected = 800.0 - ltr_ok_right;
                assert!(
                    (rtl_ok_left - expected).abs() <= f64::from(f32::EPSILON) * expected.abs(),
                    "{rtl_ok_left} is not {expected}"
                );
            });
        }

        #[test]
        fn barrier_is_dismissible_with_default_parameter() {
            test_widgets(|tester| {
                let mut show = Show::new();
                show.builder = Some(sized_box_builder());
                let observer = tester
                    .cell
                    .borrow_mut()
                    .create(DatePickerObserver::default());
                {
                    let app = tester.cell.borrow();
                    let home = Material::new()
                        .child(Center::new().child(Builder::new(move |_app, context| {
                            let show = show.clone();
                            ElevatedButton::new(
                                Some(Listener::new(move |app| drop(show.show(app, context)))),
                                Some(Text::new("X")),
                            )
                            .into_widget()
                        })))
                        .into_widget();
                    let root = crate::app::MaterialApp::new()
                        .theme(m3(&app))
                        .navigator_observers([
                            observer.erase::<waft_widgets::AnyNavigatorObserver>()
                        ])
                        .home(home)
                        .into_widget();
                    drop(app);
                    tester.pump_widget(root);
                }

                // Open the dialog.
                let button = tester.element(find::by_type::<ElevatedButton>().at(0));
                let center = tester.get_center(button);
                tester.tap_at(center, 1);
                tester.pump_and_settle();
                assert_eq!(
                    tester.cell.borrow().get(&observer).date_picker_count.get(),
                    1
                );

                // Tap on the barrier.
                tester.tap_at(Offset::new(10.0, 10.0), 1);
                tester.pump_and_settle();
                assert_eq!(
                    tester.cell.borrow().get(&observer).date_picker_count.get(),
                    0
                );
            });
        }

        #[test]
        fn barrier_is_not_dismissible_with_barrier_dismissible_is_false() {
            test_widgets(|tester| {
                let mut show = Show::new();
                show.builder = Some(sized_box_builder());
                show.barrier_dismissible = false;
                let observer = tester
                    .cell
                    .borrow_mut()
                    .create(DatePickerObserver::default());
                {
                    let app = tester.cell.borrow();
                    let home = Material::new()
                        .child(Center::new().child(Builder::new(move |_app, context| {
                            let show = show.clone();
                            ElevatedButton::new(
                                Some(Listener::new(move |app| drop(show.show(app, context)))),
                                Some(Text::new("X")),
                            )
                            .into_widget()
                        })))
                        .into_widget();
                    let root = crate::app::MaterialApp::new()
                        .theme(m3(&app))
                        .navigator_observers([
                            observer.erase::<waft_widgets::AnyNavigatorObserver>()
                        ])
                        .home(home)
                        .into_widget();
                    drop(app);
                    tester.pump_widget(root);
                }

                // Open the dialog.
                let button = tester.element(find::by_type::<ElevatedButton>().at(0));
                let center = tester.get_center(button);
                tester.tap_at(center, 1);
                tester.pump_and_settle();
                assert_eq!(
                    tester.cell.borrow().get(&observer).date_picker_count.get(),
                    1
                );

                // Tap on the barrier, which shouldn't do anything this time.
                tester.tap_at(Offset::new(10.0, 10.0), 1);
                tester.pump_and_settle();
                assert_eq!(
                    tester.cell.borrow().get(&observer).date_picker_count.get(),
                    1
                );
            });
        }

        fn last_modal_barrier<R>(
            tester: &WidgetTester,
            read: impl FnOnce(&ModalBarrier) -> R,
        ) -> R {
            let barrier = *find::by_type::<ModalBarrier>()
                .evaluate(tester)
                .last()
                .expect("a ModalBarrier");
            tester.widget(barrier, read)
        }

        #[test]
        fn barrier_color() {
            test_widgets(|tester| {
                let mut show = Show::new();
                show.builder = Some(sized_box_builder());
                open_app(tester, true, "X", show.clone());

                // Open the dialog.
                tester.tap(find::text("X"));
                tester.pump_and_settle();
                assert_eq!(
                    last_modal_barrier(tester, |barrier| barrier.color),
                    Some(Colors::BLACK54)
                );

                // Dismiss the dialog.
                tester.tap_at(Offset::new(10.0, 10.0), 1);

                show.barrier_color = Some(Colors::PINK.primary());
                open_app(tester, true, "X", show);

                // Open the dialog.
                tester.tap(find::text("X"));
                tester.pump_and_settle();
                assert_eq!(
                    last_modal_barrier(tester, |barrier| barrier.color),
                    Some(Colors::PINK.primary())
                );
            });
        }

        #[test]
        fn barrier_label() {
            test_widgets(|tester| {
                let mut show = Show::new();
                show.builder = Some(sized_box_builder());
                show.barrier_label = Some("Custom Label".to_string());
                open_app(tester, true, "X", show);

                // Open the dialog.
                tester.tap(find::text("X"));
                tester.pump_and_settle();
                assert_eq!(
                    last_modal_barrier(tester, |barrier| barrier.semantics_label.clone()),
                    Some("Custom Label".to_string())
                );
            });
        }

        #[test]
        fn uses_nested_navigator_if_use_root_navigator_is_false() {
            test_widgets(|tester| {
                let (root_observer, nested_observer) = {
                    let mut app = tester.cell.borrow_mut();
                    (
                        app.create(DatePickerObserver::default()),
                        app.create(DatePickerObserver::default()),
                    )
                };
                let root = {
                    let app = tester.cell.borrow();
                    crate::app::MaterialApp::new()
                        .theme(m3(&app))
                        .navigator_observers([
                            root_observer.erase::<waft_widgets::AnyNavigatorObserver>()
                        ])
                        .home(
                            waft_widgets::Navigator::new()
                                .observers([
                                    nested_observer.erase::<waft_widgets::AnyNavigatorObserver>()
                                ])
                                .on_generate_route(move |app, _settings| {
                                    let route = crate::page::MaterialPageRoute::new(
                                        app,
                                        Rc::new(move |_app, context| {
                                            let mut show = Show::new();
                                            show.builder = Some(sized_box_builder());
                                            show.use_root_navigator = false;
                                            ElevatedButton::new(
                                                Some(Listener::new(move |app| {
                                                    drop(show.show(app, context));
                                                })),
                                                Some(Text::new("Show Date Picker")),
                                            )
                                            .into_widget()
                                        }),
                                    );
                                    Some(route.erase())
                                }),
                        )
                        .into_widget()
                };
                tester.pump_widget(root);

                // Open the dialog.
                let button = tester.element(find::by_type::<ElevatedButton>().at(0));
                let center = tester.get_center(button);
                tester.tap_at(center, 1);

                let app = tester.cell.borrow();
                assert_eq!(app.get(&root_observer).date_picker_count.get(), 0);
                assert_eq!(app.get(&nested_observer).date_picker_count.get(), 1);
            });
        }

        #[test]
        fn honors_dialog_theme_for_shape_and_elevation() {
            test_widgets(|tester| {
                // Test that the defaults work
                open_app(tester, false, "X", Show::new());
                tester.tap(find::text("X"));
                tester.pump_and_settle();
                let (shape, elevation) = tester
                    .widget(dialog_material(tester), |material: &Material| {
                        (material.shape.clone(), material.elevation)
                    });
                assert_eq!(shape, Some(rounded(4.0)));
                assert_eq!(elevation, 24.0);

                // Test that it honors ThemeData.dialogTheme settings
                let custom_dialog_theme = crate::dialog_theme::DialogThemeData::new()
                    .shape(RoundedRectangleBorder::new(
                        BorderSide::NONE,
                        BorderRadiusGeometry::circular(40.0),
                    ))
                    .elevation(50.0);
                let theme = ThemeData::fallback(&tester.cell.borrow(), Some(false))
                    .copy_with()
                    .dialog_theme(custom_dialog_theme.clone())
                    .build();
                open_app_with_theme(tester, theme, "X", Show::new());
                tester.pump(); // start theme animation
                tester.pump_by(std::time::Duration::from_secs(5)); // end theme animation
                let (shape, elevation) = tester
                    .widget(dialog_material(tester), |material: &Material| {
                        (material.shape.clone(), material.elevation)
                    });
                assert_eq!(shape, custom_dialog_theme.shape.clone());
                assert_eq!(Some(elevation), custom_dialog_theme.elevation);
            });
        }

        fn text_button_of(tester: &WidgetTester, text: &str) -> AnyElement {
            tester.element(
                find::ancestor(
                    find::text(text),
                    find::by_type::<crate::text_button::TextButton>(),
                )
                .first(),
            )
        }

        fn animated_container(tester: &WidgetTester) -> AnyElement {
            tester.element(find::by_type::<waft_widgets::AnimatedContainer>().at(0))
        }

        fn pump_icon_app(
            tester: &mut WidgetTester,
            material3: bool,
            initial_entry_mode: DatePickerEntryMode,
            switch_to_input_entry_mode_icon: Option<fn() -> Icon>,
            switch_to_calendar_entry_mode_icon: Option<fn() -> Icon>,
        ) {
            let mut show = Show::new();
            show.initial_entry_mode = initial_entry_mode;
            show.switch_to_input_entry_mode_icon = switch_to_input_entry_mode_icon;
            show.switch_to_calendar_entry_mode_icon = switch_to_calendar_entry_mode_icon;
            open_app(tester, material3, "Click X", show);
            tester.pump_and_settle();
            let button = tester.element(find::by_type::<ElevatedButton>().at(0));
            let center = tester.get_center(button);
            tester.tap_at(center, 1);
            tester.pump_and_settle();
        }

        fn dismiss_with_ok(tester: &mut WidgetTester) {
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
        }

        #[test]
        fn honors_switch_to_input_entry_mode_icon() {
            test_widgets(|tester| {
                pump_icon_app(tester, false, DatePickerEntryMode::Calendar, None, None);
                assert_eq!(find::by_icon(&Icons::edit()).evaluate(tester).len(), 1);
                dismiss_with_ok(tester);

                pump_icon_app(tester, true, DatePickerEntryMode::Calendar, None, None);
                assert_eq!(
                    find::by_icon(&Icons::edit_outlined())
                        .evaluate(tester)
                        .len(),
                    1
                );
                dismiss_with_ok(tester);

                pump_icon_app(
                    tester,
                    false,
                    DatePickerEntryMode::Calendar,
                    Some(|| Icon::new(Some(Icons::keyboard()))),
                    None,
                );
                assert_eq!(find::by_icon(&Icons::keyboard()).evaluate(tester).len(), 1);
                dismiss_with_ok(tester);
            });
        }

        #[test]
        fn honors_switch_to_calendar_entry_mode_icon() {
            test_widgets(|tester| {
                pump_icon_app(tester, false, DatePickerEntryMode::Input, None, None);
                assert_eq!(
                    find::by_icon(&Icons::calendar_today())
                        .evaluate(tester)
                        .len(),
                    1
                );
                dismiss_with_ok(tester);

                pump_icon_app(tester, true, DatePickerEntryMode::Input, None, None);
                assert_eq!(
                    find::by_icon(&Icons::calendar_today())
                        .evaluate(tester)
                        .len(),
                    1
                );
                dismiss_with_ok(tester);

                pump_icon_app(
                    tester,
                    false,
                    DatePickerEntryMode::Input,
                    None,
                    Some(|| Icon::new(Some(Icons::favorite()))),
                );
                assert_eq!(find::by_icon(&Icons::favorite()).evaluate(tester).len(), 1);
                dismiss_with_ok(tester);
            });
        }

        fn ok_cancel_positions(tester: &WidgetTester) -> (f64, f64, f64) {
            (
                tester.get_rect(find::text("OK")).right,
                tester.get_rect(find::text("OK")).left,
                tester.get_rect(find::text("CANCEL")).right,
            )
        }

        #[test]
        fn ok_cancel_button_layout() {
            test_widgets(|tester| {
                let build_frame = |tester: &mut WidgetTester, text_direction: TextDirection| {
                    let mut show = Show::new();
                    show.builder = Some(directionality_builder(text_direction));
                    let mut options = show;
                    options.initial_entry_mode = DatePickerEntryMode::Calendar;
                    open_app(tester, false, "X", options);
                };
                let show_and_read = |tester: &mut WidgetTester, text_direction: TextDirection| {
                    build_frame(tester, text_direction);
                    tester.tap(find::text("X"));
                    tester.pump_and_settle();
                    let positions = ok_cancel_positions(tester);
                    tester.tap(find::text("OK"));
                    tester.pump_and_settle();
                    positions
                };

                // Default landscape layout.
                assert_eq!(
                    show_and_read(tester, TextDirection::Ltr),
                    (622.0, 594.0, 560.0)
                );
                assert_eq!(
                    show_and_read(tester, TextDirection::Rtl),
                    (206.0, 178.0, 324.0)
                );

                // Portrait layout (Dart's 900x1200 physical pixels at the default pixel ratio of 3).
                let tester = &mut tester_of_size(Size::new(300.0, 400.0));
                assert_eq!(
                    show_and_read(tester, TextDirection::Ltr),
                    (258.0, 230.0, 196.0)
                );
                assert_eq!(
                    show_and_read(tester, TextDirection::Rtl),
                    (70.0, 42.0, 188.0)
                );
            });
        }

        fn month_button_present(tester: &WidgetTester, prefix: &str) -> bool {
            find::by_type::<crate::icon_button::IconButton>()
                .evaluate(tester)
                .into_iter()
                .any(|button| {
                    tester
                        .widget(button, |button: &crate::icon_button::IconButton| {
                            button.tooltip.clone()
                        })
                        .is_some_and(|tooltip| tooltip.starts_with(prefix))
                })
        }

        fn tap_next_month(tester: &mut WidgetTester) {
            crate::calendar_date_picker::tests::tap_month_button(tester, "Next month");
            tester.pump_and_settle();
        }

        fn tap_previous_month(tester: &mut WidgetTester) {
            crate::calendar_date_picker::tests::tap_month_button(tester, "Previous month");
            tester.pump_and_settle();
        }

        /// Prepares the picker, runs `steps` and taps OK: what the picker completes with.
        fn pick(
            tester: &mut WidgetTester,
            options: DatePickerOptions,
            steps: impl FnOnce(&mut WidgetTester),
        ) -> Option<DateTime> {
            let result = prepare_date_picker(tester, &options);
            steps(tester);
            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            picked_date(&result)
        }

        fn prepared(
            tester: &mut WidgetTester,
            options: &DatePickerOptions,
            steps: impl FnOnce(&mut WidgetTester),
        ) {
            prepare_date_picker(tester, options);
            steps(tester);
        }

        fn pump_zero(tester: &mut WidgetTester) {
            tester.pump();
        }

        #[test]
        fn can_select_a_month() {
            test_widgets(|tester| {
                let date = pick(tester, DatePickerOptions::new(), |tester| {
                    tap_previous_month(tester);
                    tester.tap(find::text("25"));
                });
                assert_eq!(date, Some(DateTime::utc(2015, 12, 25)));
            });
        }

        #[test]
        fn can_select_a_year() {
            test_widgets(|tester| {
                prepared(tester, &DatePickerOptions::new(), |tester| {
                    tester.tap(find::text("January 2016")); // Switch to year mode.
                    pump_zero(tester);
                    tester.tap(find::text("2018"));
                    pump_zero(tester);
                    expect(tester, &find::text("January 2018"), finds_one_widget());
                });
            });
        }

        #[test]
        fn can_select_a_month_with_no_initial_date() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = None;
                let date = pick(tester, options, |tester| {
                    tap_previous_month(tester);
                    tester.tap(find::text("25"));
                });
                assert_eq!(date, Some(DateTime::utc(2015, 12, 25)));
            });
        }

        #[test]
        fn can_select_a_year_with_no_initial_date() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = None;
                prepared(tester, &options, |tester| {
                    tester.tap(find::text("January 2016")); // Switch to year mode.
                    pump_zero(tester);
                    tester.tap(find::text("2018"));
                    pump_zero(tester);
                    expect(tester, &find::text("January 2018"), finds_one_widget());
                });
            });
        }

        #[test]
        fn selecting_date_does_not_change_displayed_month() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2020, 3, 15));
                prepared(tester, &options, |tester| {
                    tap_next_month(tester);
                    expect(tester, &find::text("April 2020"), finds_one_widget());
                    tester.tap(find::text("25"));
                    tester.pump_and_settle();
                    expect(tester, &find::text("April 2020"), finds_one_widget());
                    // There isn't a 31 in April so there shouldn't be one if it is showing April
                    expect(tester, &find::text("31"), finds_nothing());
                });
            });
        }

        #[test]
        fn changing_year_does_change_selected_date() {
            test_widgets(|tester| {
                let date = pick(tester, DatePickerOptions::new(), |tester| {
                    tester.tap(find::text("January 2016"));
                    pump_zero(tester);
                    tester.tap(find::text("2018"));
                    pump_zero(tester);
                });
                assert_eq!(date, Some(DateTime::utc(2018, 1, 15)));
            });
        }

        #[test]
        fn changing_year_does_not_change_the_month() {
            test_widgets(|tester| {
                prepared(tester, &DatePickerOptions::new(), |tester| {
                    tap_next_month(tester);
                    tap_next_month(tester);
                    tester.tap(find::text("March 2016"));
                    tester.pump_and_settle();
                    tester.tap(find::text("2018"));
                    tester.pump_and_settle();
                    expect(tester, &find::text("March 2018"), finds_one_widget());
                });
            });
        }

        #[test]
        fn can_select_a_year_and_then_a_day() {
            test_widgets(|tester| {
                let date = pick(tester, DatePickerOptions::new(), |tester| {
                    tester.tap(find::text("January 2016")); // Switch to year mode.
                    pump_zero(tester);
                    tester.tap(find::text("2017"));
                    pump_zero(tester);
                    tester.tap(find::text("19"));
                });
                assert_eq!(date, Some(DateTime::utc(2017, 1, 19)));
            });
        }

        #[test]
        fn current_year_is_visible_in_year_picker() {
            test_widgets(|tester| {
                prepared(tester, &DatePickerOptions::new(), |tester| {
                    tester.tap(find::text("January 2016")); // Switch to year mode.
                    pump_zero(tester);
                    expect(tester, &find::text("2016"), finds_one_widget());
                });
            });
        }

        #[test]
        fn cannot_select_a_day_outside_bounds() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                let initial = DateTime::utc(2017, 1, 15);
                options.initial_date = Some(initial);
                options.first_date = initial;
                options.last_date = initial;
                let date = pick(tester, options, |tester| {
                    // Earlier than firstDate. Should be ignored.
                    tester.tap(find::text("10"));
                    // Later than lastDate. Should be ignored.
                    tester.tap(find::text("20"));
                });
                // We should still be on the initial date.
                assert_eq!(date, Some(initial));
            });
        }

        #[test]
        fn cannot_select_a_month_past_last_date() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2017, 1, 15));
                options.first_date = DateTime::utc(2017, 1, 15);
                options.last_date = DateTime::utc(2017, 2, 20);
                prepared(tester, &options, |tester| {
                    tap_next_month(tester);
                    // Shouldn't be possible to keep going into March.
                    assert!(!month_button_present(tester, "Next month"));
                });
            });
        }

        #[test]
        fn cannot_select_a_month_before_first_date() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2017, 1, 15));
                options.first_date = DateTime::utc(2016, 12, 10);
                options.last_date = DateTime::utc(2017, 1, 15);
                prepared(tester, &options, |tester| {
                    tap_previous_month(tester);
                    // Shouldn't be possible to keep going into November.
                    assert!(!month_button_present(tester, "Previous month"));
                });
            });
        }

        #[test]
        fn cannot_select_disabled_year() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2018, 7, 4));
                options.first_date = DateTime::utc(2018, 6, 9);
                options.last_date = DateTime::utc(2018, 12, 15);
                let date = pick(tester, options, |tester| {
                    tester.tap(find::text("July 2018")); // Switch to year mode.
                    tester.pump_and_settle();
                    tester.tap(find::text("2016")); // Disabled, doesn't change the year.
                    tester.pump_and_settle();
                });
                assert_eq!(date, Some(DateTime::utc(2018, 7, 4)));
            });
        }

        #[test]
        fn selecting_first_date_year_respects_first_date() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2018, 5, 4));
                options.first_date = DateTime::utc(2016, 6, 9);
                options.last_date = DateTime::utc(2019, 1, 15);
                prepared(tester, &options, |tester| {
                    tester.tap(find::text("May 2018"));
                    tester.pump_and_settle();
                    tester.tap(find::text("2016"));
                    tester.pump_and_settle();
                    // Month should be clamped to June as the range starts at June 2016
                    expect(tester, &find::text("June 2016"), finds_one_widget());
                });
            });
        }

        #[test]
        fn selecting_last_date_year_respects_last_date() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2018, 5, 4));
                options.first_date = DateTime::utc(2016, 6, 9);
                options.last_date = DateTime::utc(2019, 1, 15);
                prepared(tester, &options, |tester| {
                    tester.tap(find::text("May 2018"));
                    tester.pump_and_settle();
                    tester.tap(find::text("2019"));
                    tester.pump_and_settle();
                    // Month should be clamped to January as the range ends at January 2019
                    expect(tester, &find::text("January 2019"), finds_one_widget());
                });
            });
        }

        #[test]
        fn can_select_initial_calendar_picker_mode() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2014, 1, 15));
                options.initial_calendar_mode = DatePickerMode::Year;
                prepared(tester, &options, |tester| {
                    pump_zero(tester);
                    // 2018 wouldn't be available if the year picker wasn't showing.
                    // The initial current year is 2014.
                    tester.tap(find::text("2018"));
                    pump_zero(tester);
                    expect(tester, &find::text("January 2018"), finds_one_widget());
                });
            });
        }

        /// Dart's `findDayGridMaterial`: what the Material all days paint on paints, found from an
        /// arbitrary day.
        fn day_grid_material_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
            tester.nearest_material_ops(find::text("17").evaluate(tester)[0])
        }

        /// Dart's `findDayDecoration(tester, day).color`.
        fn day_decoration_color(tester: &WidgetTester, day: &str) -> Option<waft_embedder::Color> {
            let ink = tester.element(
                find::ancestor(
                    find::text(day),
                    find::by_type::<crate::ink_decoration::Ink>(),
                )
                .first(),
            );
            tester.widget(ink, |ink: &crate::ink_decoration::Ink| {
                ink.decoration
                    .as_deref()
                    .and_then(|decoration| {
                        decoration
                            .as_any()
                            .downcast_ref::<waft_painting::ShapeDecoration>()
                    })
                    .and_then(|decoration| decoration.color.as_ref())
                    .map(|color| color.color())
            })
        }

        fn pump_picker_with(tester: &mut WidgetTester, use_material3: bool) {
            let mut options = DatePickerOptions::new();
            options.use_material3 = use_material3;
            prepare_date_picker(tester, &options);
        }

        fn theme_of(tester: &WidgetTester, use_material3: bool) -> ThemeData {
            if use_material3 {
                m3(&tester.cell.borrow())
            } else {
                m2(&tester.cell.borrow())
            }
        }

        /// Asserts Dart's `paints..circle()..circle()[..circle()]..circle(color: overlay)`: today's
        /// outline, the selected day, (when pressed, the hover highlight,) then the overlay.
        fn assert_day_overlay(tester: &WidgetTester, pressed: bool, overlay: waft_embedder::Color) {
            let mut calls = vec![PaintCall::circle(None), PaintCall::circle(None)];
            if pressed {
                calls.push(PaintCall::circle(None));
            }
            calls.push(PaintCall::circle(Some(overlay)));
            assert!(
                paints(&day_grid_material_ops(tester), &calls).is_ok(),
                "the day grid's Material does not paint {calls:?}"
            );
        }

        fn hover_day_30(tester: &mut WidgetTester) {
            let center = tester.get_center(find::text("30").evaluate(tester)[0]);
            let mut pointer = TestPointer::mouse(1, 1);
            tester.add_pointer(&mut pointer, Offset::ZERO);
            tester.move_to(&mut pointer, center, false);
            tester.pump_and_settle();
        }

        fn press_day_30(tester: &mut WidgetTester) -> TestPointer {
            let center = tester.get_center(find::text("30").evaluate(tester)[0]);
            let mut pointer = TestPointer::mouse(1, 1);
            tester.add_pointer(&mut pointer, Offset::ZERO);
            tester.down(&mut pointer, center);
            tester.pump_and_settle();
            pointer
        }

        fn select_day_30(tester: &mut WidgetTester, theme: &ThemeData) {
            tester.tap(find::text("30"));
            tester.pump_and_settle();
            assert_eq!(
                day_decoration_color(tester, "30"),
                Some(theme.color_scheme.primary)
            );
        }

        #[test]
        fn current_date_is_highlighted() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.today = DateTime::utc(2016, 1, 2);
                prepare_date_picker(tester, &options);
                pump_zero(tester);
                let today_color = waft_embedder::Color::new(0xff2196f3); // default primary color
                // The current day should be painted with a circle outline.
                assert_paints(
                    &day_grid_material_ops(tester),
                    &[PaintCall::stroked_circle(today_color, 1.0)],
                );
            });
        }

        #[allow(deprecated)]
        fn day_overlay_hovered(tester: &mut WidgetTester, use_material3: bool) {
            pump_picker_with(tester, use_material3);
            let theme = theme_of(tester, use_material3);
            hover_day_30(tester);
            assert_day_overlay(
                tester,
                false,
                theme.color_scheme.on_surface_variant().with_opacity(0.08),
            );
        }

        #[allow(deprecated)]
        fn day_overlay_focused(tester: &mut WidgetTester, use_material3: bool, opacity: f64) {
            pump_picker_with(tester, use_material3);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = theme_of(tester, use_material3);
            // Navigate to the grid.
            tabs(tester, 4);
            // Navigate to day 30.
            for (physical, logical) in [
                (
                    PhysicalKeyboardKey::ARROW_DOWN,
                    LogicalKeyboardKey::ARROW_DOWN,
                ),
                (
                    PhysicalKeyboardKey::ARROW_DOWN,
                    LogicalKeyboardKey::ARROW_DOWN,
                ),
                (
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT,
                ),
            ] {
                press(tester, physical, logical);
            }
            tester.pump_and_settle();
            assert_day_overlay(
                tester,
                false,
                theme
                    .color_scheme
                    .on_surface_variant()
                    .with_opacity(opacity),
            );
        }

        #[allow(deprecated)]
        fn day_overlay_pressed(tester: &mut WidgetTester, use_material3: bool, opacity: f64) {
            pump_picker_with(tester, use_material3);
            let theme = theme_of(tester, use_material3);
            let pointer = press_day_30(tester);
            assert_day_overlay(
                tester,
                true,
                theme
                    .color_scheme
                    .on_surface_variant()
                    .with_opacity(opacity),
            );
            tester.up(&pointer);
        }

        #[allow(deprecated)]
        fn day_overlay_selected_and_hovered(tester: &mut WidgetTester, use_material3: bool) {
            pump_picker_with(tester, use_material3);
            let theme = theme_of(tester, use_material3);
            select_day_30(tester, &theme);
            hover_day_30(tester);
            assert_day_overlay(
                tester,
                false,
                theme.color_scheme.on_primary.with_opacity(0.08),
            );
        }

        #[allow(deprecated)]
        fn day_overlay_selected_and_focused(
            tester: &mut WidgetTester,
            use_material3: bool,
            opacity: f64,
        ) {
            pump_picker_with(tester, use_material3);
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let theme = theme_of(tester, use_material3);
            select_day_30(tester, &theme);
            // Navigate to the grid.
            tabs(tester, 4);
            tester.pump_and_settle();
            // Day 30 is selected and focused.
            assert_day_overlay(
                tester,
                false,
                theme.color_scheme.on_primary.with_opacity(opacity),
            );
        }

        #[allow(deprecated)]
        fn day_overlay_selected_and_pressed(
            tester: &mut WidgetTester,
            use_material3: bool,
            opacity: f64,
        ) {
            pump_picker_with(tester, use_material3);
            let theme = theme_of(tester, use_material3);
            select_day_30(tester, &theme);
            let pointer = press_day_30(tester);
            assert_day_overlay(
                tester,
                true,
                theme.color_scheme.on_primary.with_opacity(opacity),
            );
            tester.up(&pointer);
        }

        #[test]
        fn date_picker_day_overlay_color_resolves_hovered_state() {
            test_widgets(|tester| day_overlay_hovered(tester, true));
        }

        #[test]
        fn date_picker_day_overlay_color_resolves_focused_state() {
            test_widgets(|tester| day_overlay_focused(tester, true, 0.10));
        }

        #[test]
        fn date_picker_day_overlay_color_resolves_pressed_state() {
            test_widgets(|tester| day_overlay_pressed(tester, true, 0.10));
        }

        // Regression test for https://github.com/flutter/flutter/issues/130586.
        #[test]
        fn date_picker_day_overlay_color_resolves_selected_and_hovered_state() {
            test_widgets(|tester| day_overlay_selected_and_hovered(tester, true));
        }

        #[test]
        fn date_picker_day_overlay_color_resolves_selected_and_focused_state() {
            test_widgets(|tester| day_overlay_selected_and_focused(tester, true, 0.10));
        }

        #[test]
        fn date_picker_day_overlay_color_resolves_selected_and_pressed_state() {
            test_widgets(|tester| day_overlay_selected_and_pressed(tester, true, 0.10));
        }

        #[test]
        fn material2_date_picker_day_overlay_color_resolves_hovered_state() {
            test_widgets(|tester| day_overlay_hovered(tester, false));
        }

        #[test]
        fn material2_date_picker_day_overlay_color_resolves_focused_state() {
            test_widgets(|tester| day_overlay_focused(tester, false, 0.12));
        }

        #[test]
        fn material2_date_picker_day_overlay_color_resolves_pressed_state() {
            test_widgets(|tester| day_overlay_pressed(tester, false, 0.12));
        }

        #[test]
        fn material2_date_picker_day_overlay_color_resolves_selected_and_hovered_state() {
            test_widgets(|tester| day_overlay_selected_and_hovered(tester, false));
        }

        #[test]
        fn material2_date_picker_day_overlay_color_resolves_selected_and_focused_state() {
            test_widgets(|tester| day_overlay_selected_and_focused(tester, false, 0.12));
        }

        #[test]
        fn material2_date_picker_day_overlay_color_resolves_selected_and_pressed_state() {
            test_widgets(|tester| day_overlay_selected_and_pressed(tester, false, 0.38));
        }

        #[test]
        fn selecting_date_does_not_switch_picker_to_year_selection() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.initial_date = Some(DateTime::utc(2020, 5, 10));
                options.initial_calendar_mode = DatePickerMode::Year;
                prepared(tester, &options, |tester| {
                    pump_zero(tester);
                    tester.tap(find::text("2017"));
                    pump_zero(tester);
                    expect(tester, &find::text("May 2017"), finds_one_widget());
                    tester.tap(find::text("10"));
                    pump_zero(tester);
                    expect(tester, &find::text("May 2017"), finds_one_widget());
                    expect(tester, &find::text("2017"), finds_nothing());
                });
            });
        }

        fn pump_dialog_m3(tester: &mut WidgetTester, dialog: DatePickerDialog) {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new().child(dialog).into_widget(),
            ));
        }

        fn layout_dialog() -> DatePickerDialog {
            DatePickerDialog::new(DateTime::utc(2001, 1, 1), DateTime::utc(2031, 12, 31))
                .initial_date(DateTime::utc(2016, 1, 15))
        }

        /// Dart's `tester.view.physicalSize = size` at the default pixel ratio of 3.
        fn tester_of_physical_size(physical_size: Size) -> WidgetTester {
            WidgetTester::with_physical_size(
                waft_test::TestPlatform::new().on(TargetPlatform::Android),
                physical_size,
            )
        }

        fn top_right(tester: &WidgetTester, element: AnyElement) -> Offset {
            let rect = tester.get_rect(element);
            Offset::new(rect.right, rect.top)
        }

        #[test]
        fn default_calendar_mode_layout_landscape() {
            test_widgets(|_| {
                let tester = &mut tester_of_physical_size(Size::new(1920.0, 1080.0));
                pump_dialog_m3(tester, layout_dialog());

                let help_text = tester.element(find::text("Select date"));
                let header_text = tester.element(find::text("Fri, Jan 15"));
                let sub_header_text = tester.element(find::text("January 2016"));
                expect(tester, &find::text("Cancel"), finds_one_widget());
                expect(tester, &find::text("OK"), finds_one_widget());
                let inset_padding = EdgeInsets::symmetric(24.0, 16.0);

                // Test help text position.
                let container = animated_container(tester);
                let dialog_top_left = top_left(tester, container);
                let help_text_top_left = top_left(tester, help_text);
                assert_f32_equals(
                    help_text_top_left.dx(),
                    dialog_top_left.dx() + (inset_padding.horizontal() / 2.0),
                );
                assert_f32_equals(help_text_top_left.dy(), dialog_top_left.dy() + 16.0);

                // Test header text position.
                let header_text_top_left = top_left(tester, header_text);
                let help_text_bottom_left = bottom_left(tester, help_text);
                assert_f32_equals(
                    header_text_top_left.dx(),
                    dialog_top_left.dx() + (inset_padding.horizontal() / 2.0),
                );
                assert_f32_equals(header_text_top_left.dy(), help_text_bottom_left.dy() + 16.0);

                // Test switch button position.
                let switch_button = icon_button_of(tester, &Icons::edit_outlined());
                let switch_button_top_left = top_left(tester, switch_button);
                let switch_button_bottom_left = bottom_left(tester, switch_button);
                let header_text_bottom_left = bottom_left(tester, header_text);
                let dialog_bottom_left = bottom_left(tester, container);
                assert_f32_equals(switch_button_top_left.dx(), dialog_top_left.dx() + 8.0);
                assert_f32_equals(switch_button_top_left.dy(), header_text_bottom_left.dy());
                assert_f32_equals(switch_button_bottom_left.dx(), dialog_top_left.dx() + 8.0);
                assert_f32_equals(
                    switch_button_bottom_left.dy(),
                    dialog_bottom_left.dy() - 6.0,
                );

                // Test vertical divider position.
                let divider =
                    tester.element(find::by_type::<crate::divider::VerticalDivider>().at(0));
                let divider_top_left = top_left(tester, divider);
                let header_text_top_right = top_right(tester, header_text);
                assert_f32_equals(divider_top_left.dx(), header_text_top_right.dx() + 16.0);
                assert_f32_equals(divider_top_left.dy(), dialog_top_left.dy());

                // Test sub header text position.
                let sub_header_text_top_left = top_left(tester, sub_header_text);
                let divider_top_right = top_right(tester, divider);
                assert_f32_equals(sub_header_text_top_left.dx(), divider_top_right.dx() + 24.0);
                assert_f32_equals(sub_header_text_top_left.dy(), dialog_top_left.dy() + 16.0);

                // Test sub header icon position.
                let sub_header_icon =
                    tester.element(find::by_icon(&Icons::arrow_drop_down()).first());
                let sub_header_icon_top_left = top_left(tester, sub_header_icon);
                let sub_header_text_top_right = top_right(tester, sub_header_text);
                assert_f32_equals(
                    sub_header_icon_top_left.dx(),
                    sub_header_text_top_right.dx(),
                );
                assert_f32_equals(sub_header_icon_top_left.dy(), dialog_top_left.dy() + 14.0);

                // Test calendar page view position.
                let calendar_page_view =
                    tester.element(find::by_type::<waft_widgets::PageView>().at(0));
                let calendar_page_view_top_left = top_left(tester, calendar_page_view);
                let sub_header_text_bottom_left = bottom_left(tester, sub_header_text);
                assert_f32_equals(calendar_page_view_top_left.dx(), divider_top_right.dx());
                assert_f32_equals(
                    calendar_page_view_top_left.dy(),
                    sub_header_text_bottom_left.dy() + 16.0,
                );

                // Test month navigation icons position.
                let previous_month_button = icon_button_of(tester, &Icons::chevron_left());
                let next_month_button = icon_button_of(tester, &Icons::chevron_right());
                let previous_month_button_top_right = top_right(tester, previous_month_button);
                let next_month_button_top_right = top_right(tester, next_month_button);
                let dialog_top_right = top_right(tester, container);
                assert_f32_equals(
                    next_month_button_top_right.dx(),
                    dialog_top_right.dx() - 4.0,
                );
                assert_f32_equals(
                    next_month_button_top_right.dy(),
                    dialog_top_right.dy() + 2.0,
                );
                assert_f32_equals(
                    previous_month_button_top_right.dx(),
                    next_month_button_top_right.dx() - 48.0,
                );

                // Test action buttons position.
                let dialog_bottom_right = bottom_right(tester, container);
                let ok_button = text_button_of(tester, "OK");
                let cancel_button = text_button_of(tester, "Cancel");
                let ok_button_top_right = top_right(tester, ok_button);
                let cancel_button_top_right = top_right(tester, cancel_button);
                let calendar_page_view_bottom_right = bottom_right(tester, calendar_page_view);
                assert_f32_equals(ok_button_top_right.dx(), dialog_bottom_right.dx() - 8.0);
                assert_f32_equals(
                    ok_button_top_right.dy(),
                    calendar_page_view_bottom_right.dy() + 2.0,
                );
                let ok_button_top_left = top_left(tester, ok_button);
                assert_f32_equals(cancel_button_top_right.dx(), ok_button_top_left.dx() - 8.0);
            });
        }

        #[test]
        fn default_calendar_mode_layout_portrait() {
            test_widgets(|_| {
                let tester = &mut tester_of_physical_size(Size::new(1070.0, 1770.0));
                pump_dialog_m3(tester, layout_dialog());

                let help_text = tester.element(find::text("Select date"));
                let header_text = tester.element(find::text("Fri, Jan 15"));
                let sub_header_text = tester.element(find::text("January 2016"));
                expect(tester, &find::text("Cancel"), finds_one_widget());
                expect(tester, &find::text("OK"), finds_one_widget());

                // Test help text position.
                let container = animated_container(tester);
                let dialog_top_left = top_left(tester, container);
                let help_text_top_left = top_left(tester, help_text);
                assert_f32_equals(help_text_top_left.dx(), dialog_top_left.dx() + 24.0);
                assert_f32_equals(help_text_top_left.dy(), dialog_top_left.dy() + 16.0);

                // Test header text position
                let header_text_top_left = top_left(tester, header_text);
                let help_text_bottom_left = bottom_left(tester, help_text);
                assert_f32_equals(header_text_top_left.dx(), dialog_top_left.dx() + 24.0);
                assert_f32_equals(header_text_top_left.dy(), help_text_bottom_left.dy() + 28.0);

                // Test switch button position.
                let switch_button = icon_button_of(tester, &Icons::edit_outlined());
                let switch_button_top_right = top_right(tester, switch_button);
                let dialog_top_right = top_right(tester, container);
                assert_f32_equals(switch_button_top_right.dx(), dialog_top_right.dx() - 12.0);
                assert_f32_equals(
                    switch_button_top_right.dy(),
                    header_text_top_left.dy() - 4.0,
                );

                // Test horizontal divider position.
                let divider = tester.element(find::by_type::<crate::divider::Divider>().at(0));
                let divider_top_left = top_left(tester, divider);
                let header_text_bottom_left = bottom_left(tester, header_text);
                assert_f32_equals(divider_top_left.dx(), dialog_top_left.dx());
                assert_f32_equals(divider_top_left.dy(), header_text_bottom_left.dy() + 16.0);

                // Test subHeaderText position.
                let sub_header_text_top_left = top_left(tester, sub_header_text);
                let divider_bottom_left = bottom_left(tester, divider);
                assert_f32_equals(sub_header_text_top_left.dx(), dialog_top_left.dx() + 24.0);
                assert_f32_equals(
                    sub_header_text_top_left.dy(),
                    divider_bottom_left.dy() + 16.0,
                );

                // Test sub header icon position.
                let sub_header_icon =
                    tester.element(find::by_icon(&Icons::arrow_drop_down()).first());
                let sub_header_icon_top_left = top_left(tester, sub_header_icon);
                let sub_header_text_top_right = top_right(tester, sub_header_text);
                assert_f32_equals(
                    sub_header_icon_top_left.dx(),
                    sub_header_text_top_right.dx(),
                );
                assert_f32_equals(
                    sub_header_icon_top_left.dy(),
                    divider_bottom_left.dy() + 14.0,
                );

                // Test month navigation icons position.
                let previous_month_button = icon_button_of(tester, &Icons::chevron_left());
                let next_month_button = icon_button_of(tester, &Icons::chevron_right());
                let previous_month_button_top_right = top_right(tester, previous_month_button);
                let next_month_button_top_right = top_right(tester, next_month_button);
                assert_f32_equals(
                    next_month_button_top_right.dx(),
                    dialog_top_right.dx() - 4.0,
                );
                assert_f32_equals(
                    next_month_button_top_right.dy(),
                    divider_bottom_left.dy() + 2.0,
                );
                assert_f32_equals(
                    previous_month_button_top_right.dx(),
                    next_month_button_top_right.dx() - 48.0,
                );

                // Test calendar page view position.
                let calendar_page_view =
                    tester.element(find::by_type::<waft_widgets::PageView>().at(0));
                let calendar_page_view_top_left = top_left(tester, calendar_page_view);
                let sub_header_text_bottom_left = bottom_left(tester, sub_header_text);
                assert_f32_equals(calendar_page_view_top_left.dx(), dialog_top_left.dx());
                assert_f32_equals(
                    calendar_page_view_top_left.dy(),
                    sub_header_text_bottom_left.dy() + 16.0,
                );

                // Test action buttons position.
                let dialog_bottom_right = bottom_right(tester, container);
                let ok_button = text_button_of(tester, "OK");
                let cancel_button = text_button_of(tester, "Cancel");
                let ok_button_top_right = top_right(tester, ok_button);
                let cancel_button_top_right = top_right(tester, cancel_button);
                let calendar_page_view_bottom_right = bottom_right(tester, calendar_page_view);
                let ok_button_top_left = top_left(tester, ok_button);
                assert_f32_equals(ok_button_top_right.dx(), dialog_bottom_right.dx() - 8.0);
                assert_f32_equals(
                    ok_button_top_right.dy(),
                    calendar_page_view_bottom_right.dy() + 2.0,
                );
                assert_f32_equals(cancel_button_top_right.dx(), ok_button_top_left.dx() - 8.0);
            });
        }

        /// Dart's `TestCalendarDelegate` (a month of 21 or 28 days, the first on offset 1) and
        /// `_TestDateInputDelegate` (dates typed as `yyyymmdd`), each of which extends
        /// `GregorianCalendarDelegate`: everything they do not override forwards to it.
        #[derive(Debug)]
        pub(super) enum TestDelegate {
            Calendar,
            DateInput,
        }

        impl CalendarDelegate for TestDelegate {
            fn now(&self) -> DateTime {
                GregorianCalendarDelegate.now()
            }

            fn date_only(&self, date: &DateTime) -> DateTime {
                GregorianCalendarDelegate.date_only(date)
            }

            fn month_delta(&self, start_date: &DateTime, end_date: &DateTime) -> i32 {
                GregorianCalendarDelegate.month_delta(start_date, end_date)
            }

            fn add_months_to_month_date(
                &self,
                month_date: &DateTime,
                months_to_add: i32,
            ) -> DateTime {
                GregorianCalendarDelegate.add_months_to_month_date(month_date, months_to_add)
            }

            fn add_days_to_date(&self, date: &DateTime, days: i32) -> DateTime {
                GregorianCalendarDelegate.add_days_to_date(date, days)
            }

            fn first_day_offset(
                &self,
                year: i32,
                month: i32,
                localizations: &dyn MaterialLocalizations,
            ) -> i32 {
                match self {
                    TestDelegate::Calendar => 1,
                    TestDelegate::DateInput => {
                        GregorianCalendarDelegate.first_day_offset(year, month, localizations)
                    }
                }
            }

            fn get_days_in_month(&self, year: i32, month: i32) -> i32 {
                match self {
                    TestDelegate::Calendar => {
                        if month % 2 == 0 {
                            21
                        } else {
                            28
                        }
                    }
                    TestDelegate::DateInput => {
                        GregorianCalendarDelegate.get_days_in_month(year, month)
                    }
                }
            }

            fn get_month(&self, year: i32, month: i32) -> DateTime {
                GregorianCalendarDelegate.get_month(year, month)
            }

            fn get_day(&self, year: i32, month: i32, day: i32) -> DateTime {
                GregorianCalendarDelegate.get_day(year, month, day)
            }

            fn format_month_year(
                &self,
                date: &DateTime,
                localizations: &dyn MaterialLocalizations,
            ) -> String {
                GregorianCalendarDelegate.format_month_year(date, localizations)
            }

            fn format_medium_date(
                &self,
                date: &DateTime,
                localizations: &dyn MaterialLocalizations,
            ) -> String {
                GregorianCalendarDelegate.format_medium_date(date, localizations)
            }

            fn format_short_month_day(
                &self,
                date: &DateTime,
                localizations: &dyn MaterialLocalizations,
            ) -> String {
                GregorianCalendarDelegate.format_short_month_day(date, localizations)
            }

            fn format_short_date(
                &self,
                date: &DateTime,
                localizations: &dyn MaterialLocalizations,
            ) -> String {
                GregorianCalendarDelegate.format_short_date(date, localizations)
            }

            fn format_full_date(
                &self,
                date: &DateTime,
                localizations: &dyn MaterialLocalizations,
            ) -> String {
                GregorianCalendarDelegate.format_full_date(date, localizations)
            }

            fn format_compact_date(
                &self,
                date: &DateTime,
                localizations: &dyn MaterialLocalizations,
            ) -> String {
                match self {
                    TestDelegate::Calendar => {
                        GregorianCalendarDelegate.format_compact_date(date, localizations)
                    }
                    TestDelegate::DateInput => {
                        format!("{:04}{:02}{:02}", date.year(), date.month(), date.day())
                    }
                }
            }

            fn parse_compact_date(
                &self,
                input_string: Option<&str>,
                localizations: &dyn MaterialLocalizations,
            ) -> Option<DateTime> {
                match self {
                    TestDelegate::Calendar => {
                        GregorianCalendarDelegate.parse_compact_date(input_string, localizations)
                    }
                    TestDelegate::DateInput => {
                        let input = input_string.filter(|input| input.len() == 8)?;
                        let year: i32 = input.get(0..4)?.parse().ok()?;
                        let month: i32 = input.get(4..6)?.parse().ok()?;
                        let day: i32 = input.get(6..8)?.parse().ok()?;
                        let date = DateTime::utc(year, month, day);
                        (date.year() == year && date.month() == month && date.day() == day)
                            .then_some(date)
                    }
                }
            }

            fn date_help_text(&self, localizations: &dyn MaterialLocalizations) -> String {
                match self {
                    TestDelegate::Calendar => {
                        GregorianCalendarDelegate.date_help_text(localizations)
                    }
                    TestDelegate::DateInput => "yyyymmdd".to_string(),
                }
            }

            fn keyboard_input_formatters(
                &self,
                _localizations: &dyn MaterialLocalizations,
            ) -> Option<Vec<waft_services::TextInputFormatterRef>> {
                match self {
                    TestDelegate::Calendar => None,
                    TestDelegate::DateInput => Some(vec![
                        waft_services::FilteringTextInputFormatter::digits_only(),
                        waft_services::TextInputFormatterRef(Rc::new(
                            waft_services::LengthLimitingTextInputFormatter::new(Some(8)),
                        )),
                    ]),
                }
            }
        }

        #[test]
        fn keyboard_can_toggle_to_calendar_entry_mode() {
            test_widgets(|tester| {
                prepare_date_picker(tester, &DatePickerOptions::new());
                assert_eq!(text_field_count(tester), 0);
                // Navigate to the entry toggle button and activate it
                tabs(tester, 5);
                space(tester);
                tester.pump_and_settle();
                // Should be in the input mode
                assert_eq!(text_field_count(tester), 1);
            });
        }

        #[test]
        fn keyboard_can_toggle_to_year_mode() {
            test_widgets(|tester| {
                prepare_date_picker(tester, &DatePickerOptions::new());
                expect(tester, &find::text("2016"), finds_nothing());
                // Navigate to the year selector and activate it
                tab(tester);
                space(tester);
                tester.pump_and_settle();
                // The years should be visible
                expect(tester, &find::text("2016"), finds_one_widget());
            });
        }

        #[test]
        fn keyboard_can_navigate_next_previous_months() {
            test_widgets(|tester| {
                prepare_date_picker(tester, &DatePickerOptions::new());
                expect(tester, &find::text("January 2016"), finds_one_widget());
                // Navigate to the previous month button and activate it twice
                tabs(tester, 2);
                space(tester);
                tester.pump_and_settle();
                space(tester);
                tester.pump_and_settle();
                // Should be showing Nov 2015
                expect(tester, &find::text("November 2015"), finds_one_widget());

                // Navigate to the next month button and activate it four times
                tab(tester);
                for _ in 0..4 {
                    space(tester);
                    tester.pump_and_settle();
                }
                // Should be on Mar 2016
                expect(tester, &find::text("March 2016"), finds_one_widget());
            });
        }

        #[test]
        fn keyboard_can_navigate_date_grid_with_arrow_keys() {
            test_widgets(|tester| {
                let result = prepare_date_picker(tester, &DatePickerOptions::new());
                // Navigate to the grid
                tabs(tester, 4);

                // Navigate from Jan 15 to Jan 18 with arrow keys
                arrows(
                    tester,
                    &[
                        Arrow::Left,
                        Arrow::Up,
                        Arrow::Left,
                        Arrow::Down,
                        Arrow::Down,
                        Arrow::Left,
                        Arrow::Left,
                    ],
                );
                tester.pump_and_settle();

                // Activate it
                space(tester);
                tester.pump_and_settle();

                // Navigate out of the grid and to the OK button
                tabs(tester, 3);

                // Activate OK
                space(tester);
                tester.pump_and_settle();

                // Should have selected Jan 18
                assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 1, 18)));
            });
        }

        #[test]
        fn keyboard_navigating_with_arrow_keys_scrolls_months() {
            test_widgets(|tester| {
                let result = prepare_date_picker(tester, &DatePickerOptions::new());
                // Navigate to the grid
                tabs(tester, 4);
                tester.pump_and_settle();

                // Navigate from Jan 15 to Dec 31 with arrow keys
                arrows(tester, &[Arrow::Up, Arrow::Up, Arrow::Left]);
                tester.pump_and_settle();

                // Should have scrolled to Dec 2015
                expect(tester, &find::text("December 2015"), finds_one_widget());

                // Navigate from Dec 31 to Nov 26 with arrow keys
                arrows(tester, &[Arrow::Up; 5]);
                tester.pump_and_settle();

                // Should have scrolled to Nov 2015
                expect(tester, &find::text("November 2015"), finds_one_widget());

                // Activate it
                space(tester);
                tester.pump_and_settle();

                // Navigate out of the grid and to the OK button
                tabs(tester, 3);
                tester.pump_and_settle();

                // Activate OK
                space(tester);
                tester.pump_and_settle();

                assert_eq!(picked_date(&result), Some(DateTime::utc(2015, 11, 26)));
            });
        }

        #[test]
        fn keyboard_rtl_text_direction_reverses_the_horizontal_arrow_key_navigation() {
            test_widgets(|tester| {
                let mut options = DatePickerOptions::new();
                options.builder = Some(directionality_builder(TextDirection::Rtl));
                let result = prepare_date_picker(tester, &options);
                // Navigate to the grid
                tabs(tester, 4);
                tester.pump_and_settle();

                // Navigate from Jan 15 to 19 with arrow keys
                arrows(
                    tester,
                    &[
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Down,
                        Arrow::Left,
                    ],
                );
                tester.pump_and_settle();

                // Activate it
                space(tester);
                tester.pump_and_settle();

                // Navigate out of the grid and to the OK button
                tabs(tester, 3);
                tester.pump_and_settle();

                // Activate OK
                space(tester);
                tester.pump_and_settle();

                assert_eq!(picked_date(&result), Some(DateTime::utc(2016, 1, 19)));
            });
        }

        fn input_group_options() -> DatePickerOptions {
            let mut options = DatePickerOptions::new();
            options.first_date = DateTime::utc(2015, 1, 1);
            options.last_date = DateTime::utc(2017, 12, 31);
            options.initial_date = Some(DateTime::utc(2016, 1, 15));
            options.initial_entry_mode = DatePickerEntryMode::Input;
            options
        }

        fn text_field_decoration(tester: &WidgetTester) -> crate::input_decorator::InputDecoration {
            let field = tester.element(find::by_type::<TextField>().at(0));
            tester.widget(field, |field: &TextField| {
                field.decoration.clone().expect("a decoration")
            })
        }

        #[test]
        fn material3_input_mode_default_input_decoration() {
            test_widgets(|tester| {
                let mut options = input_group_options();
                options.use_material3 = true;
                prepare_date_picker(tester, &options);
                let decoration = text_field_decoration(tester);
                assert_eq!(
                    decoration.border,
                    Some(crate::input_border::InputBorderRef::new(
                        crate::input_border::OutlineInputBorder::default()
                    ))
                );
                assert_eq!(decoration.filled, Some(false));
                assert_eq!(decoration.hint_text.as_deref(), Some("mm/dd/yyyy"));
                assert_eq!(decoration.label_text.as_deref(), Some("Enter Date"));
                assert_eq!(decoration.error_text, None);
            });
        }

        #[test]
        fn material2_input_mode_default_input_decoration() {
            test_widgets(|tester| {
                prepare_date_picker(tester, &input_group_options());
                let decoration = text_field_decoration(tester);
                assert_eq!(
                    decoration.border,
                    Some(crate::input_border::InputBorderRef::new(
                        crate::input_border::UnderlineInputBorder::default()
                    ))
                );
                assert_eq!(decoration.filled, Some(false));
                assert_eq!(decoration.hint_text.as_deref(), Some("mm/dd/yyyy"));
                assert_eq!(decoration.label_text.as_deref(), Some("Enter Date"));
                assert_eq!(decoration.error_text, None);
            });
        }

        #[test]
        fn input_mode_keyboard_type_is_used() {
            test_widgets(|tester| {
                let mut options = input_group_options();
                options.keyboard_type = Some(TextInputType::TEXT);
                prepare_date_picker(tester, &options);
                let field = tester.element(find::by_type::<TextField>().at(0));
                let keyboard_type = tester.widget(field, |field: &TextField| field.keyboard_type);
                assert_eq!(keyboard_type, Some(TextInputType::TEXT));
            });
        }

        #[test]
        fn input_mode_invalid_entered_text_shows_error_on_autovalidate() {
            test_widgets(|tester| {
                prepare_date_picker(tester, &input_group_options());
                enter_text(tester, "");

                // Enter some text to trigger autovalidate.
                enter_text(tester, "xyz");
                tester.tap(find::text("OK"));
                tester.pump_and_settle();

                // Invalid format validation error should be shown.
                expect(tester, &find::text("Invalid format."), finds_one_widget());

                // Clear the text.
                enter_text(tester, "");

                // Enter an invalid date that is too long while autovalidate is still on.
                enter_text(tester, "10/05/2023666777889");
                pump_zero(tester);

                // Invalid format validation error should be shown.
                expect(tester, &find::text("Invalid format."), finds_one_widget());
            });
        }

        #[test]
        fn landscape_input_only_date_picker_headers_use_headline_small_in_portrait() {
            test_widgets_sized(Size::new(1070.0, 1770.0), |tester| {
                let mut options = DatePickerOptions::new();
                options.initial_entry_mode = DatePickerEntryMode::Input;
                options.use_material3 = true;
                prepare_date_picker(tester, &options);
                assert_eq!(header_font_size(tester, "Fri, Jan 15"), Some(32.0));
                tester.tap(find::text("Cancel"));
                tester.pump_and_settle();
            });
        }

        #[test]
        fn landscape_input_only_date_picker_headers_use_headline_small_in_landscape() {
            test_widgets_sized(Size::new(1770.0, 1070.0), |tester| {
                let mut options = DatePickerOptions::new();
                options.initial_entry_mode = DatePickerEntryMode::Input;
                options.use_material3 = true;
                prepare_date_picker(tester, &options);
                assert_eq!(header_font_size(tester, "Fri, Jan 15"), Some(24.0));
                tester.tap(find::text("Cancel"));
                tester.pump_and_settle();
            });
        }

        /// A regression test for https://github.com/flutter/flutter/issues/131989.
        #[test]
        fn dialog_contents_do_not_overflow_when_resized_during_orientation_change() {
            test_widgets(|_| {
                // Initial window size is wide for landscape mode.
                let tester = &mut tester_of_size(WIDE_WINDOW_SIZE);
                let mut options = DatePickerOptions::new();
                options.use_material3 = true;
                prepare_date_picker(tester, &options);
                assert_eq!(
                    dialog_container_size(tester),
                    CALENDAR_LANDSCAPE_DIALOG_SIZE
                );

                // Change window size to narrow for portrait mode.
                resize_view(tester, NARROW_WINDOW_SIZE);
                tester.pump();
                tester.pump_and_settle();
                assert_eq!(
                    dialog_container_size(tester),
                    CALENDAR_PORTRAIT_DIALOG_SIZE_M3
                );
            });
        }

        #[test]
        fn screen_configurations_show_and_close_the_picker_without_failing() {
            test_widgets(|_| {
                // Common screen size roughly based on a Pixel 1, and a small one based on a LG K130.
                for size in [
                    Size::new(1070.0, 1770.0),
                    Size::new(1770.0, 1070.0),
                    Size::new(320.0, 521.0),
                    Size::new(521.0, 320.0),
                ] {
                    let tester = &mut tester_of_size(size);
                    let result = prepare_date_picker(tester, &DatePickerOptions::new());
                    tester.tap(find::text("OK"));
                    tester.pump_and_settle();
                    assert_eq!(
                        picked_date(&result),
                        Some(DateTime::utc(2016, 1, 15)),
                        "{size:?}"
                    );
                }
            });
        }

        #[test]
        fn date_picker_dialog_with_updated_inset_padding() {
            test_widgets(|tester| {
                pump_dialog_m3(
                    tester,
                    layout_dialog().inset_padding(EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0)),
                );
                let dialog = tester.element(find::by_type::<Dialog>().at(0));
                let inset_padding = tester.widget(dialog, |dialog: &Dialog| dialog.inset_padding);
                assert_eq!(
                    inset_padding,
                    Some(EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0))
                );
            });
        }

        #[test]
        fn date_picker_dialog_renders_at_zero_area() {
            test_widgets(|tester| {
                let theme = m3(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(SizedBox::shrink().child(DatePickerDialog::new(
                            DateTime::utc(2001, 1, 1),
                            DateTime::utc(2031, 12, 31),
                        )))
                        .into_widget(),
                ));
                let dialog = tester.element(find::by_type::<DatePickerDialog>().at(0));
                assert_eq!(tester.get_size(dialog), Size::new(0.0, 0.0));
            });
        }

        fn last_day_text(tester: &WidgetTester) -> Option<String> {
            let inks = find::by_type::<crate::ink_decoration::Ink>().evaluate(tester);
            let day = find::by_type::<Text>()
                .evaluate(tester)
                .into_iter()
                .rfind(|text| {
                    tester
                        .ancestors(*text)
                        .iter()
                        .any(|ancestor| inks.contains(ancestor))
                })?;
            tester.widget(day, |text: &Text| text.data.clone())
        }

        #[test]
        fn displays_calendar_based_on_the_calendar_delegate() {
            test_widgets(|tester| {
                pump_dialog_m3(
                    tester,
                    layout_dialog().calendar_delegate(Rc::new(TestDelegate::Calendar)),
                );
                let next_month_button = icon_button_of(tester, &Icons::chevron_right());

                expect(tester, &find::text("January 2016"), finds_one_widget());
                assert_eq!(last_day_text(tester).as_deref(), Some("28"));

                let center = tester.get_center(next_month_button);
                tester.tap_at(center, 1);
                tester.pump_and_settle();
                expect(tester, &find::text("February 2016"), finds_one_widget());
                assert_eq!(last_day_text(tester).as_deref(), Some("21"));

                let center = tester.get_center(next_month_button);
                tester.tap_at(center, 1);
                tester.pump_and_settle();
                expect(tester, &find::text("March 2016"), finds_one_widget());
                assert_eq!(last_day_text(tester).as_deref(), Some("28"));
            });
        }

        fn open_date_input_picker(tester: &mut WidgetTester) -> super::PickerResult {
            let result: super::PickerResult = Rc::default();
            let mut show = Show::new();
            show.initial_entry_mode = DatePickerEntryMode::Input;
            show.calendar_delegate = Rc::new(TestDelegate::DateInput);
            show.dates = Some((
                DateTime::utc(2016, 1, 15),
                DateTime::utc(2001, 1, 1),
                DateTime::utc(2031, 12, 31),
            ));
            show.result = Some(Rc::clone(&result));
            open_app(tester, false, "Open", show);
            tester.tap(find::text("Open"));
            tester.pump_and_settle();
            result
        }

        #[test]
        fn date_text_input_formatter_applies_input_formatters_and_allows_text_entry_parsing() {
            test_widgets(|tester| {
                let result = open_date_input_picker(tester);

                enter_text(tester, "20240620");
                pump_zero(tester);

                tester.tap(find::text("OK"));
                tester.pump_and_settle();

                assert_eq!(picked_date(&result), Some(DateTime::utc(2024, 6, 20)));
            });
        }

        #[test]
        fn date_text_input_formatter_filters_non_digit_input_via_input_formatters() {
            test_widgets(|tester| {
                open_date_input_picker(tester);

                enter_text(tester, "2024-06-20");
                pump_zero(tester);

                // Verify that hyphens were removed by the formatter.
                assert_eq!(text_field_controller_text(tester), "20240620");
            });
        }

        #[test]
        fn date_text_input_formatter_rejects_alphabetic_characters_via_input_formatters() {
            test_widgets(|tester| {
                open_date_input_picker(tester);

                enter_text(tester, "20ab24cd0620");
                pump_zero(tester);

                // Only digits should remain in the controller.
                assert_eq!(text_field_controller_text(tester), "20240620");
            });
        }
    }

    mod range {
        //! date_range_picker_test.dart: the range picker's layout, its dialog properties, the selection
        //! of a range, the validation of typed dates, the keyboard navigation and the calendar
        //! delegates.

        use waft_widgets::testing::assert_f32_equals;

        use std::rc::Rc;

        use waft_embedder::{Size, TargetPlatform};
        use waft_foundation::{App, DateTime, Listener};
        use waft_painting::{
            BorderRadiusGeometry, BorderSide, EdgeInsets, RoundedRectangleBorder, ShapeBorderRef,
            TextDirection,
        };
        use waft_services::TextInputType;
        use waft_widgets::{AnyElement, BuildContext, Builder, Center, Icon, IntoWidget, Text};

        use super::*;
        use super::{
            PickerResult, enter_text_in_field, picked_range, range, resize_view, tap_icon,
            tester_of_size, text_field_count,
        };
        use crate::colors::Colors;
        use crate::date::{DateTimeRange, GregorianCalendarDelegate};
        use crate::dialog::Dialog;
        use crate::elevated_button::ElevatedButton;
        use crate::icons::Icons;
        use crate::material::Material;
        use crate::test_support::{PaintInspection, material_app};

        use crate::text_field::TextField;
        use crate::theme_data::ThemeData;
        use crate::tooltip::tests::{m2, m3};
        use waft_rendering::testing::{PaintCall, assert_paints};

        /// The variables of date_range_picker_test.dart's `setUp`, and `preparePicker`'s arguments.
        #[derive(Clone)]
        struct Prepare {
            first_date: DateTime,
            last_date: DateTime,
            current_date: Option<DateTime>,
            initial_date_range: Option<DateTimeRange>,
            initial_entry_mode: DatePickerEntryMode,
            cancel_text: Option<String>,
            confirm_text: Option<String>,
            error_invalid_range_text: Option<String>,
            error_format_text: Option<String>,
            error_invalid_text: Option<String>,
            field_start_hint_text: Option<String>,
            field_end_hint_text: Option<String>,
            field_start_label_text: Option<String>,
            field_end_label_text: Option<String>,
            help_text: Option<String>,
            save_text: Option<String>,
            text_direction: TextDirection,
            use_material3: bool,
            selectable_day_predicate: Option<SelectableDayForRangePredicate>,
            calendar_delegate: Rc<dyn CalendarDelegate>,
            switch_to_input_entry_mode_icon: Option<fn() -> Icon>,
            switch_to_calendar_entry_mode_icon: Option<fn() -> Icon>,
        }

        impl Prepare {
            fn new() -> Prepare {
                Prepare {
                    first_date: DateTime::utc(2015, 1, 1),
                    last_date: DateTime::utc(2016, 12, 31),
                    current_date: None,
                    initial_date_range: Some(range((2016, 1, 15), (2016, 1, 25))),
                    initial_entry_mode: DatePickerEntryMode::Calendar,
                    cancel_text: None,
                    confirm_text: None,
                    error_invalid_range_text: None,
                    error_format_text: None,
                    error_invalid_text: None,
                    field_start_hint_text: None,
                    field_end_hint_text: None,
                    field_start_label_text: None,
                    field_end_label_text: None,
                    help_text: None,
                    save_text: None,
                    text_direction: TextDirection::Ltr,
                    use_material3: false,
                    selectable_day_predicate: None,
                    calendar_delegate: Rc::new(GregorianCalendarDelegate),
                    switch_to_input_entry_mode_icon: None,
                    switch_to_calendar_entry_mode_icon: None,
                }
            }

            /// The `setUp` of the 'Input mode' groups.
            fn input_mode() -> Prepare {
                let mut prepare = Prepare::new();
                prepare.first_date = DateTime::utc(2015, 1, 1);
                prepare.last_date = DateTime::utc(2017, 12, 31);
                prepare.initial_date_range = Some(range((2017, 1, 15), (2017, 1, 17)));
                prepare.initial_entry_mode = DatePickerEntryMode::Input;
                prepare
            }

            fn show(&self, app: &mut App, context: BuildContext) -> CompleterFuture<RouteResult> {
                let text_direction = self.text_direction;
                show_date_range_picker(
                    app,
                    context,
                    self.initial_date_range,
                    self.first_date,
                    self.last_date,
                    self.current_date,
                    self.initial_entry_mode,
                    self.help_text.clone(),
                    self.cancel_text.clone(),
                    self.confirm_text.clone(),
                    self.save_text.clone(),
                    self.error_format_text.clone(),
                    self.error_invalid_text.clone(),
                    self.error_invalid_range_text.clone(),
                    self.field_start_hint_text.clone(),
                    self.field_end_hint_text.clone(),
                    self.field_start_label_text.clone(),
                    self.field_end_label_text.clone(),
                    None,
                    true,
                    None,
                    None,
                    true,
                    None,
                    None,
                    Some(Rc::new(move |_app, _context, child| {
                        waft_widgets::Directionality::new(
                            text_direction,
                            child.expect("a dialog child").clone(),
                        )
                        .into_widget()
                    })),
                    TextInputType::DATETIME,
                    self.switch_to_input_entry_mode_icon.map(|icon| icon()),
                    self.switch_to_calendar_entry_mode_icon.map(|icon| icon()),
                    self.selectable_day_predicate.clone(),
                    Rc::clone(&self.calendar_delegate),
                )
            }
        }

        /// Dart's `preparePicker`: an app with a button, tapped, that shows the date range picker.
        fn prepare_picker(tester: &mut WidgetTester, prepare: &Prepare) -> PickerResult {
            let theme = if prepare.use_material3 {
                m3(&tester.cell.borrow())
            } else {
                m2(&tester.cell.borrow())
            };
            prepare_picker_in(tester, prepare, theme)
        }

        /// [`prepare_picker`] in an app of `theme`.
        fn prepare_picker_in(
            tester: &mut WidgetTester,
            prepare: &Prepare,
            theme: ThemeData,
        ) -> PickerResult {
            let result: PickerResult = Rc::default();
            let launcher = {
                let (prepare, result) = (prepare.clone(), Rc::clone(&result));
                Material::new()
                    .child(Builder::new(move |_app, context| {
                        let (prepare, result) = (prepare.clone(), Rc::clone(&result));
                        ElevatedButton::new(
                            Some(Listener::new(move |app| {
                                *result.borrow_mut() = Some(prepare.show(app, context));
                            })),
                            Some(Text::new("Go")),
                        )
                        .into_widget()
                    }))
                    .into_widget()
            };
            tester.pump_widget(material_app(theme, launcher));
            tester.tap(find::text("Go"));
            tester.pump_and_settle();
            result
        }

        fn default_size() -> Size {
            Size::new(800.0, 600.0)
        }

        fn first_text(tester: &WidgetTester, text: &str) -> AnyElement {
            *find::text(text)
                .evaluate(tester)
                .first()
                .unwrap_or_else(|| panic!("a Text showing {text:?}"))
        }

        fn tap_first(tester: &mut WidgetTester, text: &str) {
            let element = first_text(tester, text);
            let center = tester.get_center(element);
            tester.tap_at(center, 1);
        }

        fn pick_range(
            prepare: &Prepare,
            size: Size,
            steps: impl FnOnce(&mut WidgetTester),
            action: &str,
        ) -> Option<DateTimeRange> {
            let tester = &mut tester_of_size(size);
            let result = prepare_picker(tester, prepare);
            steps(tester);
            tester.tap(find::text(action));
            tester.pump_and_settle();
            picked_range(&result)
        }

        #[test]
        fn range_save_and_help_text_is_used_in_the_dialog() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.help_text = Some("help".to_string());
                prepare.save_text = Some("make it so".to_string());
                prepare_picker(tester, &prepare);
                expect(tester, &find::text("help"), finds_one_widget());
                expect(tester, &find::text("make it so"), finds_one_widget());
            });
        }

        #[test]
        fn long_help_text_does_not_cutoff_the_save_button() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                let help_text = "long helpText".repeat(100);
                prepare.help_text = Some(help_text.clone());
                prepare.save_text = Some("make it so".to_string());
                prepare_picker(tester, &prepare);
                expect(tester, &find::text(&help_text), finds_one_widget());
                expect(tester, &find::text("make it so"), finds_one_widget());
            });
        }

        #[test]
        fn material3_has_sentence_case_labels() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);
                expect(tester, &find::text("Save"), finds_one_widget());
                expect(tester, &find::text("Select range"), finds_one_widget());
            });
        }

        #[test]
        fn initial_date_is_the_default() {
            test_widgets(|_| {
                let result = pick_range(&Prepare::new(), default_size(), |_tester| {}, "SAVE");
                assert_eq!(result, Some(range((2016, 1, 15), (2016, 1, 25))));
            });
        }

        #[test]
        fn last_month_header_should_be_visible_if_last_date_is_selected() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.initial_date_range =
                    Some(DateTimeRange::new(prepare.last_date, prepare.last_date));
                prepare_picker(tester, &prepare);
                // December header should be showing, but no November
                expect(tester, &find::text("December 2016"), finds_one_widget());
                expect(tester, &find::text("November 2016"), finds_nothing());
            });
        }

        #[test]
        fn first_month_header_should_be_visible_if_first_date_is_selected() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.initial_date_range =
                    Some(DateTimeRange::new(prepare.first_date, prepare.first_date));
                prepare_picker(tester, &prepare);
                // January and February headers should be showing, but no March
                expect(tester, &find::text("January 2015"), finds_one_widget());
                expect(tester, &find::text("February 2015"), finds_one_widget());
                expect(tester, &find::text("March 2015"), finds_nothing());
            });
        }

        #[test]
        fn current_month_header_should_be_visible_if_no_date_is_selected() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.current_date = Some(DateTime::utc(2016, 9, 1));
                prepare.initial_date_range = None;
                prepare_picker(tester, &prepare);
                // September and October headers should be showing, but no August
                expect(tester, &find::text("September 2016"), finds_one_widget());
                expect(tester, &find::text("October 2016"), finds_one_widget());
                expect(tester, &find::text("August 2016"), finds_nothing());
            });
        }

        #[test]
        fn can_cancel() {
            test_widgets(|tester| {
                let result = prepare_picker(tester, &Prepare::new());
                tap_icon(tester, &Icons::close());
                assert!(is_completed(&result));
                assert_eq!(picked_range(&result), None);
            });
        }

        #[test]
        fn can_select_a_range() {
            test_widgets(|_| {
                let result = pick_range(
                    &Prepare::new(),
                    default_size(),
                    |tester| {
                        tap_first(tester, "12");
                        tap_first(tester, "14");
                    },
                    "SAVE",
                );
                assert_eq!(result, Some(range((2016, 1, 12), (2016, 1, 14))));
            });
        }

        #[test]
        fn tapping_earlier_date_resets_selected_range() {
            test_widgets(|_| {
                let result = pick_range(
                    &Prepare::new(),
                    default_size(),
                    |tester| {
                        tap_first(tester, "12");
                        tap_first(tester, "11");
                        tap_first(tester, "15");
                    },
                    "SAVE",
                );
                assert_eq!(result, Some(range((2016, 1, 11), (2016, 1, 15))));
            });
        }

        #[test]
        fn can_select_single_day_range() {
            test_widgets(|_| {
                let result = pick_range(
                    &Prepare::new(),
                    default_size(),
                    |tester| {
                        tap_first(tester, "12");
                        tap_first(tester, "12");
                    },
                    "SAVE",
                );
                assert_eq!(result, Some(range((2016, 1, 12), (2016, 1, 12))));
            });
        }

        #[test]
        fn cannot_select_a_day_outside_bounds() {
            test_widgets(|_| {
                let mut prepare = Prepare::new();
                prepare.initial_date_range = Some(range((2017, 1, 13), (2017, 1, 15)));
                prepare.first_date = DateTime::utc(2017, 1, 12);
                prepare.last_date = DateTime::utc(2017, 1, 16);
                let result = pick_range(
                    &prepare,
                    default_size(),
                    |tester| {
                        // Earlier than firstDate. Should be ignored.
                        tester.tap(find::text("10"));
                        // Later than lastDate. Should be ignored.
                        tester.tap(find::text("20"));
                    },
                    "SAVE",
                );
                // We should still be on the initial date.
                assert_eq!(result, prepare.initial_date_range);
            });
        }

        #[test]
        fn can_select_a_range_even_if_the_range_includes_non_selectable_days() {
            test_widgets(|_| {
                let mut prepare = Prepare::new();
                prepare.selectable_day_predicate = Some(Rc::new(|day, _, _| day.day() != 13));
                let result = pick_range(
                    &prepare,
                    default_size(),
                    |tester| {
                        tap_first(tester, "12");
                        tap_first(tester, "14");
                    },
                    "SAVE",
                );
                // The day 13 is not selectable, but the range is still valid.
                assert_eq!(result, Some(range((2016, 1, 12), (2016, 1, 14))));
            });
        }

        #[test]
        fn cannot_select_a_day_inside_bounds_but_not_selectable() {
            test_widgets(|_| {
                let mut prepare = Prepare::new();
                prepare.initial_date_range = Some(range((2017, 1, 13), (2017, 1, 14)));
                prepare.first_date = DateTime::utc(2017, 1, 12);
                prepare.last_date = DateTime::utc(2017, 1, 16);
                prepare.selectable_day_predicate = Some(Rc::new(|day, _, _| day.day() != 15));
                let result = pick_range(
                    &prepare,
                    default_size(),
                    |tester| {
                        // Non-selectable date. Should be ignored.
                        tester.tap(find::text("15"));
                    },
                    "SAVE",
                );
                // We should still be on the initial date.
                assert_eq!(result, prepare.initial_date_range);
            });
        }

        #[test]
        fn selectable_date_becoming_non_selectable_when_selected_start_day() {
            test_widgets(|_| {
                let mut prepare = Prepare::new();
                prepare.selectable_day_predicate =
                    Some(Rc::new(|day, selected_start, selected_end| {
                        if selected_end.is_none()
                            && let Some(selected_start) = selected_start
                        {
                            return day == selected_start || day.is_after(&selected_start);
                        }
                        true
                    }));
                let result = pick_range(
                    &prepare,
                    default_size(),
                    |tester| {
                        for day in ["12", "11", "14"] {
                            tap_first(tester, day);
                            tester.pump_and_settle();
                        }
                    },
                    "SAVE",
                );
                assert_eq!(result, Some(range((2016, 1, 12), (2016, 1, 14))));
            });
        }

        #[test]
        fn selectable_day_predicate_should_be_called_with_the_selected_start_and_end_dates() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.initial_date_range = Some(range((2017, 1, 13), (2017, 1, 15)));
                prepare.first_date = DateTime::utc(2017, 1, 12);
                prepare.last_date = DateTime::utc(2017, 1, 16);
                let calls = Rc::new(std::cell::Cell::new(0));
                prepare.selectable_day_predicate = Some({
                    let calls = Rc::clone(&calls);
                    Rc::new(move |_day, selected_start_date, selected_end_date| {
                        assert_eq!(selected_start_date, Some(DateTime::utc(2017, 1, 13)));
                        assert_eq!(selected_end_date, Some(DateTime::utc(2017, 1, 15)));
                        calls.set(calls.get() + 1);
                        true
                    })
                });
                prepare_picker(tester, &prepare);
                assert!(
                    calls.get() > 0,
                    "the predicate is asked about the days shown"
                );
            });
        }

        #[test]
        fn can_switch_from_calendar_to_input_entry_mode() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::new());
                assert_eq!(field_count(tester), 0);
                tap_icon(tester, &Icons::edit());
                assert_eq!(field_count(tester), 2);
            });
        }

        #[test]
        fn can_switch_from_input_to_calendar_entry_mode() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.initial_entry_mode = DatePickerEntryMode::Input;
                prepare_picker(tester, &prepare);
                assert_eq!(field_count(tester), 2);
                tap_icon(tester, &Icons::calendar_today());
                assert_eq!(field_count(tester), 0);
            });
        }

        #[test]
        fn can_not_switch_out_of_calendar_only_mode() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.initial_entry_mode = DatePickerEntryMode::CalendarOnly;
                prepare_picker(tester, &prepare);
                assert_eq!(field_count(tester), 0);
                assert!(find::by_icon(&Icons::edit()).evaluate(tester).is_empty());
            });
        }

        #[test]
        fn can_not_switch_out_of_input_only_mode() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.initial_entry_mode = DatePickerEntryMode::InputOnly;
                prepare_picker(tester, &prepare);
                assert_eq!(field_count(tester), 2);
                assert!(
                    find::by_icon(&Icons::calendar_today())
                        .evaluate(tester)
                        .is_empty()
                );
            });
        }

        #[test]
        fn input_only_mode_should_validate_date() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.initial_entry_mode = DatePickerEntryMode::InputOnly;
                prepare.error_invalid_text = Some("oops".to_string());
                prepare_picker(tester, &prepare);
                enter_in(tester, 0, "08/08/2014");
                enter_in(tester, 1, "08/08/2014");
                expect(tester, &find::text("oops"), finds_nothing());

                tester.tap(find::text("OK"));
                tester.pump_and_settle();
                expect(tester, &find::text("oops"), finds_n_widgets(2));
            });
        }

        #[test]
        fn switching_to_input_mode_keeps_selected_date() {
            test_widgets(|_| {
                let result = pick_range(
                    &Prepare::new(),
                    default_size(),
                    |tester| {
                        tap_first(tester, "12");
                        tap_first(tester, "14");
                        tap_icon(tester, &Icons::edit());
                    },
                    "OK",
                );
                assert_eq!(result, Some(range((2016, 1, 12), (2016, 1, 14))));
            });
        }

        #[test]
        fn material3_default_layout_calendar_mode() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);

                let help_text = tester.element(find::text("Select range"));
                let first_date_header_text = tester.element(find::text("Jan 15"));
                let last_date_header_text = tester.element(find::text("Jan 25, 2016"));
                expect(tester, &find::text("Save"), finds_one_widget());

                // Test the close button position.
                let close_button = tester.element(
                    find::ancestor(
                        find::by_type::<crate::icon_button::IconButton>().at(0),
                        find::by_type::<Center>(),
                    )
                    .first(),
                );
                let close_button_bottom_right = bottom_right(tester, close_button);
                let help_text_top_left = top_left(tester, help_text);
                assert_f32_equals(close_button_bottom_right.dx(), 56.0);
                assert_f32_equals(close_button_bottom_right.dy(), help_text_top_left.dy());

                // Test the save and entry buttons position.
                let save_button_bottom_left = bottom_left(
                    tester,
                    tester.element(find::by_type::<crate::text_button::TextButton>().at(0)),
                );
                let entry_button_bottom_left =
                    bottom_left(tester, icon_button_of(tester, &Icons::edit_outlined()));
                assert_f32_equals(save_button_bottom_left.dx(), 711.6);
                assert_f32_equals(save_button_bottom_left.dy(), help_text_top_left.dy());
                assert_f32_equals(
                    entry_button_bottom_left.dx(),
                    save_button_bottom_left.dx() - 48.0,
                );
                assert_f32_equals(entry_button_bottom_left.dy(), help_text_top_left.dy());

                // Test help text position.
                let help_text_bottom_left = bottom_left(tester, help_text);
                assert_f32_equals(help_text_bottom_left.dx(), 72.0);
                assert_f32_equals(
                    help_text_bottom_left.dy(),
                    close_button_bottom_right.dy() + 20.0,
                );

                // Test the header position.
                let first_date_header_top_left = top_left(tester, first_date_header_text);
                let last_date_header_top_left = top_left(tester, last_date_header_text);
                assert_f32_equals(first_date_header_top_left.dx(), 72.0);
                assert_f32_equals(
                    first_date_header_top_left.dy(),
                    help_text_bottom_left.dy() + 8.0,
                );
                // Dart also asserts that the last date header starts 66.0 pixels after the first one
                // ends: the width of the `Text(' – ')` between them, three glyphs. Valo leaves the
                // trailing space out of a paragraph's max intrinsic width, so that assertion is
                // deferred (`PORTING.md`) until Valo counts it.
                assert_f32_equals(
                    last_date_header_top_left.dy(),
                    help_text_bottom_left.dy() + 8.0,
                );

                // Test the day headers position.
                let day_headers_grid =
                    tester.element(find::by_type::<waft_widgets::GridView>().at(0));
                let day_headers_grid_top_left = top_left(tester, day_headers_grid);
                let first_date_header_bottom_left = bottom_left(tester, first_date_header_text);
                assert_f32_equals(day_headers_grid_top_left.dx(), (800.0 - 384.0) / 2.0);
                assert_f32_equals(
                    day_headers_grid_top_left.dy(),
                    first_date_header_bottom_left.dy() + 16.0,
                );

                // Test the calendar custom scroll view position.
                let calendar_scroll_view =
                    tester.element(find::by_type::<waft_widgets::CustomScrollView>().at(0));
                let calendar_scroll_view_top_left = top_left(tester, calendar_scroll_view);
                let day_headers_grid_bottom_left = bottom_left(tester, day_headers_grid);
                assert_f32_equals(calendar_scroll_view_top_left.dx(), 0.0);
                assert_f32_equals(
                    calendar_scroll_view_top_left.dy(),
                    day_headers_grid_bottom_left.dy(),
                );
            });
        }

        #[test]
        fn material2_default_layout_calendar_mode() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::new());

                let help_text = tester.element(find::text("SELECT RANGE"));
                let first_date_header_text = tester.element(find::text("Jan 15"));
                let last_date_header_text = tester.element(find::text("Jan 25, 2016"));
                expect(tester, &find::text("SAVE"), finds_one_widget());

                // Test the close button position.
                let close_button =
                    tester.element(find::by_type::<crate::action_buttons::CloseButton>().at(0));
                let close_button_bottom_right = bottom_right(tester, close_button);
                let help_text_top_left = top_left(tester, help_text);
                assert_f32_equals(close_button_bottom_right.dx(), 56.0);
                assert_f32_equals(
                    close_button_bottom_right.dy(),
                    help_text_top_left.dy() - 6.0,
                );

                // Test the save and entry buttons position.
                let save_button_bottom_left = bottom_left(
                    tester,
                    tester.element(find::by_type::<crate::text_button::TextButton>().at(0)),
                );
                let entry_button_bottom_left =
                    bottom_left(tester, icon_button_of(tester, &Icons::edit()));
                assert_f32_equals(save_button_bottom_left.dx(), 800.0 - 80.0);
                assert_f32_equals(save_button_bottom_left.dy(), help_text_top_left.dy() - 6.0);
                assert_f32_equals(
                    entry_button_bottom_left.dx(),
                    save_button_bottom_left.dx() - 48.0,
                );
                assert_f32_equals(entry_button_bottom_left.dy(), help_text_top_left.dy() - 6.0);

                // Test help text position.
                let help_text_bottom_left = bottom_left(tester, help_text);
                assert_f32_equals(help_text_bottom_left.dx(), 72.0);
                assert_f32_equals(
                    help_text_bottom_left.dy(),
                    close_button_bottom_right.dy() + 16.0,
                );

                // Test the header position.
                let first_date_header_top_left = top_left(tester, first_date_header_text);
                let last_date_header_top_left = top_left(tester, last_date_header_text);
                assert_f32_equals(first_date_header_top_left.dx(), 72.0);
                assert_f32_equals(
                    first_date_header_top_left.dy(),
                    help_text_bottom_left.dy() + 8.0,
                );
                // Dart also asserts that the last date header starts 72.0 pixels after the first one
                // ends: the width of the `Text(' – ')` between them, three glyphs. Valo leaves the
                // trailing space out of a paragraph's max intrinsic width, so that assertion is
                // deferred (`PORTING.md`) until Valo counts it.
                assert_f32_equals(
                    last_date_header_top_left.dy(),
                    help_text_bottom_left.dy() + 8.0,
                );

                // Test the day headers position.
                let day_headers_grid =
                    tester.element(find::by_type::<waft_widgets::GridView>().at(0));
                let day_headers_grid_top_left = top_left(tester, day_headers_grid);
                let first_date_header_bottom_left = bottom_left(tester, first_date_header_text);
                assert_f32_equals(day_headers_grid_top_left.dx(), (800.0 - 384.0) / 2.0);
                assert_f32_equals(
                    day_headers_grid_top_left.dy(),
                    first_date_header_bottom_left.dy() + 16.0,
                );

                // Test the calendar custom scroll view position.
                let calendar_scroll_view =
                    tester.element(find::by_type::<waft_widgets::CustomScrollView>().at(0));
                let calendar_scroll_view_top_left = top_left(tester, calendar_scroll_view);
                let day_headers_grid_bottom_left = bottom_left(tester, day_headers_grid);
                assert_f32_equals(calendar_scroll_view_top_left.dx(), 0.0);
                assert_f32_equals(
                    calendar_scroll_view_top_left.dy(),
                    day_headers_grid_bottom_left.dy(),
                );
            });
        }

        /// What the dialog's `Material` and `Dialog` say that the default-properties tests read.
        struct DialogProperties {
            color: Option<waft_embedder::Color>,
            shadow_color: Option<waft_embedder::Color>,
            surface_tint_color: Option<waft_embedder::Color>,
            elevation: f64,
            shape: Option<ShapeBorderRef>,
            clip_behavior: waft_embedder::Clip,
            inset_padding: Option<EdgeInsets>,
        }

        fn dialog_properties(tester: &WidgetTester) -> DialogProperties {
            let mut properties = tester.widget(dialog_material(tester), |material: &Material| {
                DialogProperties {
                    color: material.color,
                    shadow_color: material.shadow_color,
                    surface_tint_color: material.surface_tint_color,
                    elevation: material.elevation,
                    shape: material.shape.clone(),
                    clip_behavior: material.clip_behavior,
                    inset_padding: None,
                }
            });
            let dialog = tester.element(find::by_type::<Dialog>().at(0));
            properties.inset_padding =
                tester.widget(dialog, |dialog: &Dialog| dialog.inset_padding);
            properties
        }

        fn square_shape() -> ShapeBorderRef {
            ShapeBorderRef::new(RoundedRectangleBorder::default())
        }

        #[test]
        fn material3_default_dialog_properties_calendar_mode() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);
                let theme = m3(&tester.cell.borrow());
                let properties = dialog_properties(tester);
                assert_eq!(
                    properties.color,
                    Some(theme.color_scheme.surface_container_high())
                );
                assert_eq!(properties.shadow_color, Some(Colors::TRANSPARENT));
                assert_eq!(properties.surface_tint_color, Some(Colors::TRANSPARENT));
                assert_eq!(properties.elevation, 0.0);
                assert_eq!(properties.shape, Some(square_shape()));
                assert_eq!(properties.clip_behavior, waft_embedder::Clip::AntiAlias);
                assert_eq!(properties.inset_padding, Some(EdgeInsets::ZERO));
            });
        }

        #[test]
        fn material2_default_dialog_properties_calendar_mode() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::new());
                let theme = m2(&tester.cell.borrow());
                let properties = dialog_properties(tester);
                assert_eq!(properties.color, Some(theme.color_scheme.surface));
                assert_eq!(properties.shadow_color, Some(Colors::TRANSPARENT));
                assert_eq!(properties.surface_tint_color, Some(Colors::TRANSPARENT));
                assert_eq!(properties.elevation, 0.0);
                assert_eq!(properties.shape, Some(square_shape()));
                assert_eq!(properties.clip_behavior, waft_embedder::Clip::AntiAlias);
                assert_eq!(properties.inset_padding, Some(EdgeInsets::ZERO));
            });
        }

        #[test]
        fn material3_default_dialog_properties_input_mode() {
            test_widgets(|tester| {
                let mut prepare = Prepare::input_mode();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);
                let theme = m3(&tester.cell.borrow());
                let properties = dialog_properties(tester);
                assert_eq!(
                    properties.color,
                    Some(theme.color_scheme.surface_container_high())
                );
                assert_eq!(properties.shadow_color, Some(Colors::TRANSPARENT));
                assert_eq!(properties.surface_tint_color, Some(Colors::TRANSPARENT));
                assert_eq!(properties.elevation, 6.0);
                assert_eq!(properties.shape, Some(rounded(28.0)));
                assert_eq!(properties.clip_behavior, waft_embedder::Clip::AntiAlias);
                assert_eq!(
                    properties.inset_padding,
                    Some(EdgeInsets::symmetric(24.0, 16.0))
                );
            });
        }

        #[test]
        fn material2_default_dialog_properties_input_mode() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::input_mode());
                let theme = m2(&tester.cell.borrow());
                let properties = dialog_properties(tester);
                assert_eq!(properties.color, Some(theme.color_scheme.surface));
                assert_eq!(properties.shadow_color, Some(theme.shadow_color));
                assert_eq!(properties.surface_tint_color, None);
                assert_eq!(properties.elevation, 24.0);
                assert_eq!(properties.shape, Some(rounded(4.0)));
                assert_eq!(properties.clip_behavior, waft_embedder::Clip::AntiAlias);
                assert_eq!(
                    properties.inset_padding,
                    Some(EdgeInsets::symmetric(24.0, 16.0))
                );
            });
        }

        #[test]
        fn material3_scaffold_and_app_bar_defaults() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);
                let theme = m3(&tester.cell.borrow());

                let scaffold = tester.element(find::by_type::<crate::scaffold::Scaffold>().at(0));
                let background_color = tester
                    .widget(scaffold, |scaffold: &crate::scaffold::Scaffold| {
                        scaffold.background_color
                    });
                assert_eq!(background_color, None);

                let app_bar = tester.element(find::by_type::<crate::app_bar::AppBar>().at(0));
                let icon_theme = waft_widgets::IconThemeData::new()
                    .color(theme.color_scheme.on_surface_variant());
                let (app_bar_icon_theme, actions_icon_theme, elevation, scrolled_under, background) =
                    tester.widget(app_bar, |app_bar: &crate::app_bar::AppBar| {
                        (
                            app_bar.icon_theme.clone(),
                            app_bar.actions_icon_theme.clone(),
                            app_bar.elevation,
                            app_bar.scrolled_under_elevation,
                            app_bar.background_color.clone(),
                        )
                    });
                assert_eq!(app_bar_icon_theme, Some(icon_theme.clone()));
                assert_eq!(actions_icon_theme, Some(icon_theme));
                assert_eq!(elevation, Some(0.0));
                assert_eq!(scrolled_under, Some(0.0));
                assert_eq!(
                    background.map(|color| color.color()),
                    Some(Colors::TRANSPARENT)
                );
            });
        }

        #[test]
        fn material2_scaffold_and_app_bar_defaults() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::new());
                let theme = m2(&tester.cell.borrow());

                let scaffold = tester.element(find::by_type::<crate::scaffold::Scaffold>().at(0));
                let background_color = tester
                    .widget(scaffold, |scaffold: &crate::scaffold::Scaffold| {
                        scaffold.background_color
                    });
                assert_eq!(background_color, Some(theme.color_scheme.surface));

                let app_bar = tester.element(find::by_type::<crate::app_bar::AppBar>().at(0));
                let icon_theme =
                    waft_widgets::IconThemeData::new().color(theme.color_scheme.on_primary);
                let (app_bar_icon_theme, actions_icon_theme, elevation, scrolled_under, background) =
                    tester.widget(app_bar, |app_bar: &crate::app_bar::AppBar| {
                        (
                            app_bar.icon_theme.clone(),
                            app_bar.actions_icon_theme.clone(),
                            app_bar.elevation,
                            app_bar.scrolled_under_elevation,
                            app_bar.background_color.clone(),
                        )
                    });
                assert_eq!(app_bar_icon_theme, Some(icon_theme.clone()));
                assert_eq!(actions_icon_theme, Some(icon_theme));
                assert_eq!(elevation, None);
                assert_eq!(scrolled_under, None);
                assert_eq!(
                    background.map(|color| color.color()),
                    Some(theme.color_scheme.primary)
                );
            });
        }

        fn field_count(tester: &WidgetTester) -> usize {
            text_field_count(tester)
        }

        fn enter_in(tester: &mut WidgetTester, index: usize, text: &str) {
            enter_text_in_field(tester, index, text);
        }

        fn input_text_field_decoration(
            tester: &WidgetTester,
            index: usize,
        ) -> crate::input_decorator::InputDecoration {
            let field = tester.element(find::by_type::<TextField>().at(index));
            tester.widget(field, |field: &TextField| {
                field.decoration.clone().expect("a decoration")
            })
        }

        #[test]
        fn material3_input_mode_default_input_decoration() {
            test_widgets(|tester| {
                let mut prepare = Prepare::input_mode();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);
                for (index, label) in [(0, "Start Date"), (1, "End Date")] {
                    let decoration = input_text_field_decoration(tester, index);
                    assert_eq!(
                        decoration.border,
                        Some(crate::input_border::InputBorderRef::new(
                            crate::input_border::OutlineInputBorder::default()
                        ))
                    );
                    assert_eq!(decoration.filled, Some(false));
                    assert_eq!(decoration.hint_text.as_deref(), Some("mm/dd/yyyy"));
                    assert_eq!(decoration.label_text.as_deref(), Some(label));
                    assert_eq!(decoration.error_text, None);
                }
            });
        }

        #[test]
        fn material2_input_mode_default_input_decoration() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::input_mode());
                for (index, label) in [(0, "Start Date"), (1, "End Date")] {
                    let decoration = input_text_field_decoration(tester, index);
                    assert_eq!(
                        decoration.border,
                        Some(crate::input_border::InputBorderRef::new(
                            crate::input_border::UnderlineInputBorder::default()
                        ))
                    );
                    assert_eq!(decoration.filled, Some(false));
                    assert_eq!(decoration.hint_text.as_deref(), Some("mm/dd/yyyy"));
                    assert_eq!(decoration.label_text.as_deref(), Some(label));
                    assert_eq!(decoration.error_text, None);
                }
            });
        }

        #[test]
        fn input_mode_initial_entry_mode_is_used() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::input_mode());
                assert_eq!(field_count(tester), 2);
            });
        }

        #[test]
        fn input_mode_all_custom_strings_are_used() {
            test_widgets(|tester| {
                let mut prepare = Prepare::input_mode();
                prepare.initial_date_range = None;
                prepare.cancel_text = Some("nope".to_string());
                prepare.confirm_text = Some("yep".to_string());
                prepare.field_start_hint_text = Some("hint1".to_string());
                prepare.field_end_hint_text = Some("hint2".to_string());
                prepare.field_start_label_text = Some("label1".to_string());
                prepare.field_end_label_text = Some("label2".to_string());
                prepare.help_text = Some("help".to_string());
                prepare_picker(tester, &prepare);
                for text in ["nope", "yep", "hint1", "hint2", "label1", "label2", "help"] {
                    assert_eq!(find::text(text).evaluate(tester).len(), 1, "{text}");
                }
            });
        }

        #[test]
        fn input_mode_initial_date_range_is_the_default() {
            test_widgets(|_| {
                let result = pick_range(&Prepare::input_mode(), default_size(), |_tester| {}, "OK");
                assert_eq!(result, Some(range((2017, 1, 15), (2017, 1, 17))));
            });
        }

        #[test]
        fn input_mode_can_toggle_to_calendar_entry_mode() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::input_mode());
                assert_eq!(field_count(tester), 2);
                tap_icon(tester, &Icons::calendar_today());
                assert_eq!(field_count(tester), 0);
            });
        }

        #[test]
        fn input_mode_toggle_to_calendar_mode_keeps_selected_date() {
            test_widgets(|_| {
                let mut prepare = Prepare::input_mode();
                prepare.initial_date_range = None;
                let result = pick_range(
                    &prepare,
                    default_size(),
                    |tester| {
                        enter_in(tester, 0, "12/25/2016");
                        enter_in(tester, 1, "12/27/2016");
                        tap_icon(tester, &Icons::calendar_today());
                    },
                    "SAVE",
                );
                assert_eq!(result, Some(range((2016, 12, 25), (2016, 12, 27))));
            });
        }

        #[test]
        fn input_mode_entered_text_returns_range() {
            test_widgets(|_| {
                let mut prepare = Prepare::input_mode();
                prepare.initial_date_range = None;
                let result = pick_range(
                    &prepare,
                    default_size(),
                    |tester| {
                        enter_in(tester, 0, "12/25/2016");
                        enter_in(tester, 1, "12/27/2016");
                    },
                    "OK",
                );
                assert_eq!(result, Some(range((2016, 12, 25), (2016, 12, 27))));
            });
        }

        fn range_error_case(
            tester: &mut WidgetTester,
            set: impl FnOnce(&mut Prepare),
            start: &str,
            end: &str,
            error: &str,
            expected_count: usize,
        ) {
            let mut prepare = Prepare::input_mode();
            prepare.initial_date_range = None;
            set(&mut prepare);
            prepare_picker(tester, &prepare);
            enter_in(tester, 0, start);
            enter_in(tester, 1, end);
            expect(tester, &find::text(error), finds_nothing());

            tester.tap(find::text("OK"));
            tester.pump_and_settle();
            assert_eq!(find::text(error).evaluate(tester).len(), expected_count);
        }

        #[test]
        fn input_mode_too_short_entered_text_shows_error() {
            test_widgets(|tester| {
                range_error_case(
                    tester,
                    |prepare| prepare.error_format_text = Some("oops".to_string()),
                    "12/25",
                    "12/25",
                    "oops",
                    2,
                );
            });
        }

        #[test]
        fn input_mode_bad_format_entered_text_shows_error() {
            test_widgets(|tester| {
                range_error_case(
                    tester,
                    |prepare| prepare.error_format_text = Some("oops".to_string()),
                    "20202014",
                    "20212014",
                    "oops",
                    2,
                );
            });
        }

        #[test]
        fn input_mode_invalid_entered_text_shows_error() {
            test_widgets(|tester| {
                range_error_case(
                    tester,
                    |prepare| prepare.error_invalid_text = Some("oops".to_string()),
                    "08/08/2014",
                    "08/08/2014",
                    "oops",
                    2,
                );
            });
        }

        #[test]
        fn input_mode_end_before_start_date_shows_error() {
            test_widgets(|tester| {
                range_error_case(
                    tester,
                    |prepare| prepare.error_invalid_range_text = Some("oops".to_string()),
                    "12/27/2016",
                    "12/25/2016",
                    "oops",
                    1,
                );
            });
        }

        #[test]
        fn input_mode_error_text_only_displayed_for_invalid_date() {
            test_widgets(|tester| {
                range_error_case(
                    tester,
                    |prepare| prepare.error_invalid_text = Some("oops".to_string()),
                    "12/27/2016",
                    "01/01/2018",
                    "oops",
                    1,
                );
            });
        }

        #[test]
        fn input_mode_end_before_start_date_does_not_get_passed_to_calendar_mode() {
            test_widgets(|tester| {
                let mut prepare = Prepare::input_mode();
                prepare.initial_date_range = None;
                prepare_picker(tester, &prepare);
                enter_in(tester, 0, "12/27/2016");
                enter_in(tester, 1, "12/25/2016");

                tap_icon(tester, &Icons::calendar_today());
                tester.tap(find::text("SAVE"));
                tester.pump_and_settle();

                // Save button should be disabled, so dialog should still be up with the first date
                // selected, but no end date
                expect(tester, &find::text("Dec 27"), finds_one_widget());
                expect(tester, &find::text("End Date"), finds_one_widget());
            });
        }

        #[test]
        fn input_decoration_theme_is_honored() {
            test_widgets(|tester| {
                let border = crate::input_border::InputBorderRef::none();
                let theme = ThemeData::builder()
                    .input_decoration_theme(
                        crate::input_decorator::InputDecorationThemeData::new()
                            .border(border.clone()),
                    )
                    .build(&tester.cell.borrow());
                prepare_picker_in(tester, &Prepare::input_mode(), theme);

                // The start and the end date text fields.
                let painted =
                    crate::input_decorator::tests::common::borders_and_container_colors(tester);
                assert_eq!(painted.len(), 2);
                for (actual_border, container_color) in painted {
                    assert_eq!(actual_border, border);
                    assert_eq!(container_color, Colors::TRANSPARENT);
                }
            });
        }

        fn toggled_calendar_texts(
            tester: &mut WidgetTester,
            set: impl FnOnce(&mut Prepare),
            start: &str,
            end: &str,
        ) {
            let mut prepare = Prepare::new();
            prepare.initial_entry_mode = DatePickerEntryMode::Input;
            set(&mut prepare);
            prepare_picker(tester, &prepare);
            enter_in(tester, 0, start);
            enter_in(tester, 1, end);
            tap_icon(tester, &Icons::calendar_today());
        }

        #[test]
        fn toggle_from_input_entry_mode_validates_an_invalid_start_date() {
            test_widgets(|tester| {
                // Invalid start date should have neither a start nor end date selected in calendar
                // mode
                toggled_calendar_texts(tester, |_| {}, "12/27/1918", "12/25/2016");
                expect(tester, &find::text("Start Date"), finds_one_widget());
                expect(tester, &find::text("End Date"), finds_one_widget());
            });
        }

        #[test]
        fn toggle_from_input_entry_mode_validates_a_non_selectable_start_date() {
            test_widgets(|tester| {
                // Even if start and end dates are selected, the start date is not selectable ending up
                // to no date selected at all in calendar mode.
                toggled_calendar_texts(
                    tester,
                    |prepare| {
                        prepare.selectable_day_predicate =
                            Some(Rc::new(|day, _, _| day != DateTime::utc(2016, 12, 24)));
                    },
                    "12/24/2016",
                    "12/25/2016",
                );
                expect(tester, &find::text("Start Date"), finds_one_widget());
                expect(tester, &find::text("End Date"), finds_one_widget());
            });
        }

        #[test]
        fn toggle_from_input_entry_mode_validates_an_invalid_end_date() {
            test_widgets(|tester| {
                // Invalid end date should only have a start date selected
                toggled_calendar_texts(tester, |_| {}, "12/24/2016", "12/25/2050");
                expect(tester, &find::text("Dec 24"), finds_one_widget());
                expect(tester, &find::text("End Date"), finds_one_widget());
            });
        }

        #[test]
        fn toggle_from_input_entry_mode_validates_a_non_selectable_end_date() {
            test_widgets(|tester| {
                // The end date is not selectable, so only the start date should be selected.
                toggled_calendar_texts(
                    tester,
                    |prepare| {
                        prepare.selectable_day_predicate =
                            Some(Rc::new(|day, _, _| day != DateTime::utc(2016, 12, 25)));
                    },
                    "12/24/2016",
                    "12/25/2016",
                );
                expect(tester, &find::text("Dec 24"), finds_one_widget());
                expect(tester, &find::text("End Date"), finds_one_widget());
            });
        }

        #[test]
        fn toggle_from_input_entry_mode_validates_an_invalid_range() {
            test_widgets(|tester| {
                // Start date after end date should just use the start date
                toggled_calendar_texts(tester, |_| {}, "12/25/2016", "12/24/2016");
                expect(tester, &find::text("Dec 25"), finds_one_widget());
                expect(tester, &find::text("End Date"), finds_one_widget());
            });
        }

        #[test]
        fn range_ok_cancel_button_layout() {
            test_widgets(|tester| {
                let show_ok_cancel_dialog =
                    |tester: &mut WidgetTester, text_direction: TextDirection| {
                        let mut prepare = Prepare::new();
                        prepare.first_date = DateTime::utc(2001, 1, 1);
                        prepare.last_date = DateTime::utc(2031, 12, 31);
                        prepare.initial_date_range = None;
                        prepare.text_direction = text_direction;
                        prepare_picker(tester, &prepare);
                        tap_icon(tester, &Icons::edit());
                    };
                let dismiss = |tester: &mut WidgetTester| {
                    tester.tap(find::text("CANCEL"));
                    tester.pump_and_settle();
                };

                show_ok_cancel_dialog(tester, TextDirection::Ltr);
                assert_f32_equals(tester.get_rect(find::text("OK")).right, 622.0);
                assert_f32_equals(tester.get_rect(find::text("OK")).left, 594.0);
                assert_f32_equals(tester.get_rect(find::text("CANCEL")).right, 560.0);
                dismiss(tester);

                show_ok_cancel_dialog(tester, TextDirection::Rtl);
                assert_f32_equals(tester.get_rect(find::text("OK")).right, 206.0);
                assert_f32_equals(tester.get_rect(find::text("OK")).left, 178.0);
                assert_f32_equals(tester.get_rect(find::text("CANCEL")).right, 324.0);
                dismiss(tester);
            });
        }

        #[test]
        fn range_keyboard_can_toggle_to_input_entry_mode() {
            test_widgets(|tester| {
                prepare_picker(tester, &Prepare::new());
                assert_eq!(field_count(tester), 0);
                // Navigate to the entry toggle button and activate it
                tabs(tester, 2);
                space(tester);
                tester.pump_and_settle();
                // Should be in the input mode
                assert_eq!(field_count(tester), 2);
            });
        }

        #[test]
        fn range_keyboard_can_navigate_date_grid_with_arrow_keys() {
            test_widgets(|tester| {
                let result = prepare_picker(tester, &Prepare::new());
                // Navigate to the grid
                tabs(tester, 4);
                tester.pump_and_settle();

                // Navigate from Jan 15 to Jan 18 with arrow keys
                arrows(
                    tester,
                    &[
                        Arrow::Left,
                        Arrow::Up,
                        Arrow::Left,
                        Arrow::Down,
                        Arrow::Down,
                        Arrow::Left,
                        Arrow::Left,
                    ],
                );
                tester.pump_and_settle();

                // Activate it to select the beginning of the range
                space(tester);
                tester.pump_and_settle();

                // Navigate to Jan 29
                arrows(
                    tester,
                    &[
                        Arrow::Down,
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Right,
                    ],
                );
                tester.pump_and_settle();

                // Activate it to select the end of the range
                space(tester);
                tester.pump_and_settle();

                // Navigate out of the grid and to the OK button
                tabs(tester, 3);

                // Activate OK
                space(tester);
                tester.pump_and_settle();

                // Should have selected Jan 18 - Jan 29
                assert_eq!(
                    picked_range(&result),
                    Some(range((2016, 1, 18), (2016, 1, 29)))
                );
            });
        }

        #[test]
        fn range_keyboard_navigating_with_arrow_keys_scrolls_as_needed() {
            test_widgets(|tester| {
                let result = prepare_picker(tester, &Prepare::new());
                // Jan and Feb headers should be showing, but no March
                expect(tester, &find::text("January 2016"), finds_one_widget());
                expect(tester, &find::text("February 2016"), finds_one_widget());
                expect(tester, &find::text("March 2016"), finds_nothing());

                // Navigate to the grid
                tabs(tester, 4);

                // Navigate from Jan 15 to Jan 18 with arrow keys
                arrows(
                    tester,
                    &[
                        Arrow::Left,
                        Arrow::Left,
                        Arrow::Left,
                        Arrow::Left,
                        Arrow::Down,
                    ],
                );
                tester.pump_and_settle();

                // Activate it to select the beginning of the range
                space(tester);
                tester.pump_and_settle();

                // Navigate to Mar 17
                arrows(tester, &[Arrow::Down; 8]);
                arrows(tester, &[Arrow::Right; 3]);
                tester.pump_and_settle();

                // Jan should have scrolled off, Mar should be visible
                expect(tester, &find::text("January 2016"), finds_nothing());
                expect(tester, &find::text("February 2016"), finds_one_widget());
                expect(tester, &find::text("March 2016"), finds_one_widget());

                // Activate it to select the end of the range
                space(tester);
                tester.pump_and_settle();

                // Navigate out of the grid and to the OK button
                tabs(tester, 3);

                // Activate OK
                space(tester);
                tester.pump_and_settle();

                // Should have selected Jan 18 - Mar 17
                assert_eq!(
                    picked_range(&result),
                    Some(range((2016, 1, 18), (2016, 3, 17)))
                );
            });
        }

        #[test]
        fn range_keyboard_rtl_text_direction_reverses_the_horizontal_arrow_key_navigation() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.text_direction = TextDirection::Rtl;
                let result = prepare_picker(tester, &prepare);
                // Navigate to the grid
                tabs(tester, 4);

                // Navigate from Jan 15 to 19 with arrow keys
                arrows(
                    tester,
                    &[
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Right,
                        Arrow::Down,
                        Arrow::Left,
                    ],
                );
                tester.pump_and_settle();

                // Activate it
                space(tester);
                tester.pump_and_settle();

                // Navigate to Jan 21
                arrows(tester, &[Arrow::Left, Arrow::Left]);
                tester.pump_and_settle();

                // Activate it
                space(tester);
                tester.pump_and_settle();

                // Navigate out of the grid and to the OK button
                tabs(tester, 3);

                // Activate OK
                space(tester);
                tester.pump_and_settle();

                // Should have selected Jan 19 - Jan 21
                assert_eq!(
                    picked_range(&result),
                    Some(range((2016, 1, 19), (2016, 1, 21)))
                );
            });
        }

        /// Records the haptic feedback the platform is asked for: Dart's `FeedbackTester`.
        #[derive(Default)]
        struct FeedbackTester {
            haptics: std::cell::RefCell<Vec<waft_embedder::HapticFeedbackType>>,
        }

        impl waft_embedder::Haptics for FeedbackTester {
            fn feedback(&self, kind: waft_embedder::HapticFeedbackType) {
                self.haptics.borrow_mut().push(kind);
            }
        }

        fn feedback_tester() -> (WidgetTester, Rc<FeedbackTester>) {
            let feedback = Rc::new(FeedbackTester::default());
            let platform = waft_test::TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_haptics(Rc::clone(&feedback) as Rc<dyn waft_embedder::Haptics>);
            (WidgetTester::with_platform(platform), feedback)
        }

        fn haptic_prepare() -> Prepare {
            let mut prepare = Prepare::new();
            prepare.initial_date_range = Some(range((2017, 1, 15), (2017, 1, 17)));
            prepare.first_date = DateTime::utc(2017, 1, 10);
            prepare.last_date = DateTime::utc(2018, 1, 20);
            prepare
        }

        /// A regression test for https://github.com/flutter/flutter/issues/131989.
        #[test]
        fn dialog_contents_do_not_overflow_when_resized_from_landscape_to_portrait() {
            test_widgets(|_| {
                // Initial window size is wide for landscape mode.
                let tester = &mut tester_of_size(WIDE_WINDOW_SIZE);
                prepare_picker(tester, &Prepare::new());

                // Change window size to narrow for portrait mode.
                resize_view(tester, NARROW_WINDOW_SIZE);
                tester.pump();
            });
        }

        #[test]
        fn haptic_feedback_selecting_dates_vibrates() {
            test_widgets(|_| {
                let (mut feedback_tester, feedback) = feedback_tester();
                let tester = &mut feedback_tester;
                prepare_picker(tester, &haptic_prepare());
                let interval = std::time::Duration::from_millis(10);
                for (day, count) in [("10", 1), ("12", 2), ("14", 3)] {
                    tap_first(tester, day);
                    tester.pump_by(interval);
                    assert_eq!(feedback.haptics.borrow().len(), count, "after {day}");
                }
            });
        }

        #[test]
        fn haptic_feedback_tapping_unselectable_date_does_not_vibrate() {
            test_widgets(|_| {
                let (mut feedback_tester, feedback) = feedback_tester();
                let tester = &mut feedback_tester;
                prepare_picker(tester, &haptic_prepare());
                tap_first(tester, "8");
                tester.pump_by(std::time::Duration::from_millis(10));
                assert_eq!(feedback.haptics.borrow().len(), 0);
            });
        }

        #[test]
        fn range_picker_close_button_shape_should_be_square() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);
                let theme = m3(&tester.cell.borrow());
                let button = icon_button_of(tester, &Icons::close());
                assert_eq!(tester.get_size(button), Size::new(48.0, 48.0));

                // Test the close button overlay size is square.
                let center = tester.get_center(button);
                let mut pointer = TestPointer::mouse(1, 0);
                tester.add_pointer(&mut pointer, center);
                tester.move_to(&mut pointer, center, false);
                tester.pump_and_settle();

                #[allow(deprecated)]
                let hover_color = theme.color_scheme.on_surface_variant().with_opacity(0.08);
                let render = tester.render_object(button);
                assert_paints(
                    &tester.painted_ops_deep(render),
                    &[PaintCall::rect_at(
                        waft_embedder::Rect::from_ltrb(0.0, 0.0, 40.0, 40.0),
                        Some(hover_color),
                    )],
                );
            });
        }

        fn input_only_headline_font_size(size: Size) -> Option<f64> {
            let tester = &mut tester_of_size(size);
            let mut prepare = Prepare::new();
            prepare.initial_entry_mode = DatePickerEntryMode::Input;
            prepare.use_material3 = true;
            prepare_picker(tester, &prepare);
            let font_size = header_font_size(tester, "Jan 15 \u{2013} Jan 25, 2016");
            tester.tap(find::text("Cancel"));
            tester.pump_and_settle();
            font_size
        }

        #[test]
        fn landscape_input_only_date_picker_headers_use_headline_small_in_portrait() {
            test_widgets(|_| {
                assert_eq!(
                    input_only_headline_font_size(Size::new(1070.0, 1770.0)),
                    Some(32.0)
                );
            });
        }

        #[test]
        fn landscape_input_only_date_picker_headers_use_headline_small_in_landscape() {
            test_widgets(|_| {
                assert_eq!(
                    input_only_headline_font_size(Size::new(1770.0, 1070.0)),
                    Some(24.0)
                );
            });
        }

        fn pump_icon_app(
            tester: &mut WidgetTester,
            use_material3: bool,
            initial_entry_mode: DatePickerEntryMode,
            switch_to_input_entry_mode_icon: Option<fn() -> Icon>,
            switch_to_calendar_entry_mode_icon: Option<fn() -> Icon>,
        ) {
            let mut prepare = Prepare::new();
            prepare.use_material3 = use_material3;
            prepare.first_date = DateTime::utc(2020, 1, 1);
            prepare.last_date = DateTime::utc(2030, 1, 1);
            prepare.initial_date_range = None;
            prepare.initial_entry_mode = initial_entry_mode;
            prepare.cancel_text = Some("CANCEL".to_string());
            prepare.switch_to_input_entry_mode_icon = switch_to_input_entry_mode_icon;
            prepare.switch_to_calendar_entry_mode_icon = switch_to_calendar_entry_mode_icon;
            prepare_picker(tester, &prepare);
        }

        #[test]
        fn range_honors_switch_to_input_entry_mode_icon() {
            test_widgets(|tester| {
                pump_icon_app(tester, false, DatePickerEntryMode::Calendar, None, None);
                assert_eq!(find::by_icon(&Icons::edit()).evaluate(tester).len(), 1);
                tap_icon(tester, &Icons::close());

                pump_icon_app(tester, true, DatePickerEntryMode::Calendar, None, None);
                assert_eq!(
                    find::by_icon(&Icons::edit_outlined())
                        .evaluate(tester)
                        .len(),
                    1
                );
                tap_icon(tester, &Icons::close());

                pump_icon_app(
                    tester,
                    false,
                    DatePickerEntryMode::Calendar,
                    Some(|| Icon::new(Some(Icons::keyboard()))),
                    None,
                );
                assert_eq!(find::by_icon(&Icons::keyboard()).evaluate(tester).len(), 1);
                tap_icon(tester, &Icons::close());
            });
        }

        #[test]
        fn range_honors_switch_to_calendar_entry_mode_icon() {
            test_widgets(|tester| {
                pump_icon_app(tester, false, DatePickerEntryMode::Input, None, None);
                assert_eq!(
                    find::by_icon(&Icons::calendar_today())
                        .evaluate(tester)
                        .len(),
                    1
                );
                tester.tap(find::text("CANCEL"));
                tester.pump_and_settle();

                pump_icon_app(tester, true, DatePickerEntryMode::Input, None, None);
                assert_eq!(
                    find::by_icon(&Icons::calendar_today())
                        .evaluate(tester)
                        .len(),
                    1
                );
                tester.tap(find::text("CANCEL"));
                tester.pump_and_settle();

                pump_icon_app(
                    tester,
                    false,
                    DatePickerEntryMode::Input,
                    None,
                    Some(|| Icon::new(Some(Icons::favorite()))),
                );
                assert_eq!(find::by_icon(&Icons::favorite()).evaluate(tester).len(), 1);
                tester.tap(find::text("CANCEL"));
                tester.pump_and_settle();
            });
        }

        fn keyboard_type_of_shown_dialog(
            tester: &mut WidgetTester,
            keyboard_type: Option<TextInputType>,
        ) -> TextInputType {
            let (first_date, last_date) = dialog_first_last();
            let launcher = Material::new()
                .child(Builder::new(move |_app, context| {
                    // If no keyboardType, expect the default.
                    let keyboard_type = keyboard_type.unwrap_or(TextInputType::DATETIME);
                    ElevatedButton::new(
                        Some(Listener::new(move |app| {
                            drop(show_date_range_picker(
                                app,
                                context,
                                Some(range((2016, 1, 15), (2016, 1, 25))),
                                first_date,
                                last_date,
                                None,
                                DatePickerEntryMode::Input,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                true,
                                None,
                                None,
                                true,
                                None,
                                None,
                                None,
                                keyboard_type,
                                None,
                                None,
                                None,
                                Rc::new(GregorianCalendarDelegate),
                            ));
                        })),
                        Some(Text::new("Go")),
                    )
                    .into_widget()
                }))
                .into_widget();
            let theme = m2(&tester.cell.borrow());
            tester.pump_widget(material_app(theme, launcher));
            tester.tap(find::text("Go"));
            tester.pump_and_settle();
            let dialog = tester.element(find::by_type::<DateRangePickerDialog>().at(0));
            tester.widget(dialog, |dialog: &DateRangePickerDialog| {
                dialog.keyboard_type
            })
        }

        #[test]
        fn date_range_picker_takes_the_default_keyboard_type() {
            test_widgets(|tester| {
                assert_eq!(
                    keyboard_type_of_shown_dialog(tester, None),
                    TextInputType::DATETIME
                );
            });
        }

        #[test]
        fn date_range_picker_takes_keyboard_type_email_address() {
            test_widgets(|tester| {
                assert_eq!(
                    keyboard_type_of_shown_dialog(tester, Some(TextInputType::EMAIL_ADDRESS)),
                    TextInputType::EMAIL_ADDRESS
                );
            });
        }

        fn pump_range_dialog(tester: &mut WidgetTester, dialog: DateRangePickerDialog) {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new().child(dialog).into_widget(),
            ));
        }

        fn dialog_first_last() -> (DateTime, DateTime) {
            (DateTime::utc(2015, 1, 1), DateTime::utc(2016, 12, 31))
        }

        #[test]
        fn range_calendar_delegate_displays_calendar_based_on_the_calendar_delegate() {
            test_widgets(|tester| {
                let (first_date, last_date) = dialog_first_last();
                pump_range_dialog(
                    tester,
                    DateRangePickerDialog::new(first_date, last_date)
                        .initial_date_range(range((2016, 1, 15), (2016, 1, 25)))
                        .calendar_delegate(Rc::new(super::dialog::TestDelegate::Calendar)),
                );

                // The month a header text titles: its nearest enclosing Column.
                let day_count = |month_year: &str| {
                    let month = find::ancestor(
                        find::text(month_year),
                        find::by_type::<waft_widgets::Column>(),
                    )
                    .first();
                    find::descendant(
                        find::descendant(month, find::by_type::<crate::ink_well::InkResponse>()),
                        find::by_type::<Text>(),
                    )
                    .evaluate(tester)
                    .len()
                };
                assert_eq!(day_count("January 2016"), 28);
                assert_eq!(day_count("February 2016"), 21);
            });
        }

        /// The texts both `showDateRangePicker` delegate cases expect once each.
        fn assert_default_range_header(tester: &WidgetTester) {
            expect(tester, &find::text("Select range"), finds_one_widget());
            expect(tester, &find::text("Jan 15"), finds_one_widget());
            expect(tester, &find::text("Jan 25, 2016"), finds_one_widget());
            expect(tester, &find::text("Save"), finds_one_widget());
        }

        #[test]
        fn show_date_range_picker_uses_gregorian_calendar_delegate_by_default() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.use_material3 = true;
                prepare_picker(tester, &prepare);
                assert_default_range_header(tester);
            });
        }

        #[test]
        fn show_date_range_picker_using_custom_calendar_delegate_implementation() {
            test_widgets(|tester| {
                let mut prepare = Prepare::new();
                prepare.use_material3 = true;
                prepare.calendar_delegate = Rc::new(super::dialog::TestDelegate::Calendar);
                prepare_picker(tester, &prepare);
                assert_default_range_header(tester);
            });
        }

        #[test]
        fn date_range_picker_dialog_does_not_crash_at_zero_area() {
            test_widgets(|tester| {
                let (first_date, last_date) = dialog_first_last();
                let theme = m3(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            waft_widgets::SizedBox::shrink()
                                .child(DateRangePickerDialog::new(first_date, last_date)),
                        )
                        .into_widget(),
                ));
                let dialog = tester.element(find::by_type::<DateRangePickerDialog>().at(0));
                assert_eq!(tester.get_size(dialog), Size::new(0.0, 0.0));
            });
        }

        #[test]
        fn local_input_decoration_theme_is_honored() {
            test_widgets(|tester| {
                let (first_date, last_date) = dialog_first_last();
                let theme = m3(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            crate::input_decorator::InputDecorationTheme::new()
                                .with_data(
                                    crate::input_decorator::InputDecorationThemeData::new()
                                        .filled(true),
                                )
                                .child(
                                    DateRangePickerDialog::new(first_date, last_date)
                                        .with_current_date(DateTime::utc(2016, 1, 30))
                                        .initial_entry_mode(DatePickerEntryMode::InputOnly),
                                ),
                        )
                        .into_widget(),
                ));
                let start_date_decoration = input_text_field_decoration(tester, 0);
                assert_eq!(start_date_decoration.filled, Some(true));
            });
        }

        #[test]
        fn date_range_picker_dialog_current_date_is_optional() {
            test_widgets(|tester| {
                let (first_date, last_date) = dialog_first_last();
                let theme = m3(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            crate::input_decorator::InputDecorationTheme::new()
                                .with_data(
                                    crate::input_decorator::InputDecorationThemeData::new()
                                        .filled(true),
                                )
                                .child(
                                    DateRangePickerDialog::new(first_date, last_date)
                                        .initial_entry_mode(DatePickerEntryMode::InputOnly),
                                ),
                        )
                        .into_widget(),
                ));
                assert_eq!(field_count(tester), 2);
            });
        }

        #[test]
        fn date_range_picker_respects_date_picker_theme_day_shape() {
            test_widgets(|tester| {
                let custom_shape = waft_painting::BeveledRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(10.0),
                );
                let theme = {
                    let app = tester.cell.borrow();
                    ThemeData::builder()
                        .date_picker_theme(
                            crate::date_picker_theme::DatePickerThemeData::new().day_shape(
                                <dyn waft_widgets::WidgetStateProperty<
                                        Option<waft_painting::OutlinedBorderRef>,
                                    >>::all(Some(
                                    waft_painting::OutlinedBorderRef::new(custom_shape),
                                )),
                            ),
                        )
                        .build(&app)
                };
                let mut prepare = Prepare::new();
                prepare.first_date = DateTime::utc(2023, 1, 1);
                prepare.last_date = DateTime::utc(2024, 1, 1);
                prepare.initial_date_range = Some(range((2023, 1, 15), (2023, 1, 20)));
                let launcher = Material::new()
                    .child(Builder::new(move |_app, context| {
                        let prepare = prepare.clone();
                        ElevatedButton::new(
                            Some(Listener::new(move |app| drop(prepare.show(app, context)))),
                            Some(Text::new("Open Picker")),
                        )
                        .into_widget()
                    }))
                    .into_widget();
                tester.pump_widget(material_app(theme, launcher));
                tester.tap(find::text("Open Picker"));
                tester.pump_and_settle();

                let selected_day_text = first_text(tester, "15");
                let shapes: Vec<ShapeBorderRef> = tester
                    .ancestors(selected_day_text)
                    .into_iter()
                    .filter_map(|ancestor| {
                        let app = tester.cell.borrow();
                        let container = waft_widgets::downcast_widget::<waft_widgets::Container>(
                            &**ancestor.widget(&app),
                        )?;
                        container
                            .decoration
                            .as_deref()?
                            .as_any()
                            .downcast_ref::<waft_painting::ShapeDecoration>()
                            .map(|decoration| decoration.shape.clone())
                    })
                    .collect();
                assert_eq!(shapes.first(), Some(&ShapeBorderRef::new(custom_shape)));
            });
        }
    }
}
