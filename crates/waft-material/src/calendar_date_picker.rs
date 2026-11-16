//! Flutter counterpart: material_ui's `calendar_date_picker.dart`.
//!
//! The calendar a date picker shows: a month grid that pages by month, and a year grid that
//! picks the year. The grids are built from the [`CalendarDelegate`] and the
//! [`DatePickerThemeData`] defaults, so a day's colors, shape and text come from the ambient
//! theme, not from this file.
//!
//! Dart's `_DayPickerGridDelegate` and `_YearPickerGridDelegate` read the text scale and the
//! orientation from a `BuildContext` while laying out. A [`SliverGridDelegate`] here has no
//! [`App`], so the delegates take those two values when the widget builds them; the widgets
//! read `MediaQuery` in `build`, as Dart's layout-time read subscribes them to it.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{AnimationBehavior, AnimationController, Curves};
use waft_embedder::{Color, TargetPlatform};
use waft_foundation::{App, DateTime, Handle, Listener, Ref, ValueChanged};
use waft_gestures::DragStartBehavior;
use waft_painting::axis_direction_is_reversed;
use waft_painting::{
    BorderSide, EdgeInsetsDirectional, EdgeInsetsGeometry, OutlinedBorderRef, ShapeBorderRef,
    ShapeDecoration, TextOverflow, TextStyle,
};
use waft_rendering::{
    SliverConstraints, SliverGridDelegate, SliverGridLayout, SliverGridRegularTileLayout,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::{HapticFeedback, LogicalKeyboardKey};
use waft_widgets::{
    ActionRef, BuildContext, CallbackAction, Center, Column, DefaultTextStyle,
    DirectionalFocusIntent, Directionality, ExcludeSemantics, Expanded, Flexible, FocusNode,
    FocusNodeLeaf, FocusableActionDetector, GlobalKey, GridView, Icon, InheritedWidget, IntentRef,
    IntoWidget, KeyRef, ListenableBuilder, MediaQuery, NextFocusIntent, Orientation, Padding,
    PageController, PageView, PreviousFocusIntent, RotationTransition, Row, ScrollController,
    ScrollControllerLeaf, Semantics, ShortcutActivatorRef, ShortcutMap, SingleActivator,
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, SizedBox,
    SliverChildListDelegate, Spacer, Stack, State, StateData, StatefulWidget, Text,
    TraversalDirection, WidgetRef, WidgetState, WidgetStateProperty, WidgetStates,
    WidgetStatesController,
};

use crate::date::{CalendarDelegate, DatePickerMode, GregorianCalendarDelegate};
use crate::date_picker_theme::DatePickerTheme;
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::divider::Divider;
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::ink_decoration::Ink;
use crate::ink_well::{InkResponse, InkWell};
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::theme::Theme;
use waft_widgets::SelectableDayPredicate;

const MONTH_SCROLL_DURATION: Duration = Duration::from_millis(200);

// Current M2 implementation is not compliant with the M2 specification.
// Instead of a 42 pixels row height it should be 40 with a 2 pixels inner padding.
// See: https://m2.material.io/components/date-pickers#specs.
const DAY_PICKER_ROW_HEIGHT_M2: f64 = 42.0;
// For M3, row height is 48 pxiels with 4 pixels inner padding.
// See: https://m3.material.io/components/date-pickers/specs#2d53890e-a08f-4c63-a0d9-abd9e95b4245.
const DAY_PICKER_ROW_HEIGHT_M3: f64 = 48.0;

const MAX_DAY_PICKER_ROW_COUNT: i32 = 6; // A 31 day month that starts on Saturday.
// One extra row for the day-of-week header.
const MAX_DAY_PICKER_HEIGHT_M2: f64 =
    DAY_PICKER_ROW_HEIGHT_M2 * (MAX_DAY_PICKER_ROW_COUNT + 1) as f64;
const MAX_DAY_PICKER_HEIGHT_M3: f64 =
    DAY_PICKER_ROW_HEIGHT_M3 * (MAX_DAY_PICKER_ROW_COUNT + 1) as f64;

const MONTH_PICKER_HORIZONTAL_PADDING_PORTRAIT_M3: f64 = 12.0;
const MONTH_PICKER_HORIZONTAL_PADDING_OTHER: f64 = 8.0;

const YEAR_PICKER_COLUMN_COUNT: i32 = 3;
const YEAR_PICKER_PADDING: f64 = 16.0;
const YEAR_PICKER_ROW_HEIGHT: f64 = 52.0;
const YEAR_PICKER_ROW_SPACING: f64 = 8.0;

const SUB_HEADER_HEIGHT: f64 = 52.0;
const MONTH_NAV_BUTTONS_WIDTH: f64 = 108.0;

// 3.0 is the maximum scale factor on mobile phones. As of 07/30/24, iOS goes up
// to a max of 3.0 text scale factor, and Android goes up to 2.0. This is the
// default used for non-range date pickers. This default is changed to a lower
// value at different parts of the date pickers depending on content, and device
// orientation.
const MAX_TEXT_SCALE_FACTOR: f64 = 3.0;

const MODE_TOGGLE_BUTTON_MAX_SCALE_FACTOR: f64 = 2.0;

// The max scale factor of the day picker grid. This affects the size of the
// individual days in calendar view. Due to them filling a majority of the modal,
// which covers most of the screen, there's a limit in how large they can grow.
// There is also less room vertically in landscape orientation.
const DAY_PICKER_GRID_PORTRAIT_MAX_SCALE_FACTOR: f64 = 2.0;
const DAY_PICKER_GRID_LANDSCAPE_MAX_SCALE_FACTOR: f64 = 1.5;

// 14 is a common font size used to compute the effective text scale.
const FONT_SIZE_TO_SCALE: f64 = 14.0;

/// Signature for a function that builds the widget for a day of the calendar; Dart's
/// `CalendarDatePickerDayBuilder`.
pub type CalendarDatePickerDayBuilder =
    Rc<dyn Fn(&mut App, BuildContext, CalendarDatePickerDayDetails) -> WidgetRef>;

/// Details about a day in a [`CalendarDatePicker`], given to a
/// [`CalendarDatePickerDayBuilder`].
#[derive(Clone)]
pub struct CalendarDatePickerDayDetails {
    /// The date of the day.
    pub day: DateTime,

    /// The widget states of the day, such as selected and disabled. They also carry the
    /// interaction states of the day (hovered, focused, pressed) as they change.
    pub states: WidgetStates,

    /// The default day widget: the day's number in its decoration. Wrap it, or ignore it to
    /// build a different one.
    pub child: WidgetRef,
}

/// Signature for a function that builds the widget for a weekday header of the calendar;
/// Dart's `CalendarDatePickerWeekdayBuilder`.
pub type CalendarDatePickerWeekdayBuilder =
    Rc<dyn Fn(&mut App, BuildContext, CalendarDatePickerWeekdayDetails) -> WidgetRef>;

/// Details about a weekday header in a [`CalendarDatePicker`], given to a
/// [`CalendarDatePickerWeekdayBuilder`].
#[derive(Clone)]
pub struct CalendarDatePickerWeekdayDetails {
    /// The weekday, as [`DateTime::weekday`] numbers it: 1 for Monday to 7 for Sunday.
    pub weekday: i32,

    /// The default weekday widget: the narrow weekday name.
    pub child: WidgetRef,
}

fn vibrate(app: &App, platform: TargetPlatform) {
    match platform {
        TargetPlatform::Android
        | TargetPlatform::Fuchsia
        | TargetPlatform::Linux
        | TargetPlatform::Windows => HapticFeedback::vibrate(app),
        TargetPlatform::IOS | TargetPlatform::MacOS => {}
    }
}

/// Dart's `style?.apply(color: color)`.
fn apply_color(style: Option<TextStyle>, color: Option<Color>) -> Option<TextStyle> {
    style.map(|style| match color {
        Some(color) => style.apply().color(color).into_style(),
        None => style.apply().into_style(),
    })
}

/// Dart's `MediaQuery.textScalerOf(context).clamp(maxScaleFactor: max).scale(14) / 14`.
fn clamped_text_scale_factor(app: &mut App, context: BuildContext, max_scale_factor: f64) -> f64 {
    MediaQuery::text_scaler_of(app, context)
        .clamp(0.0, max_scale_factor)
        .scale(FONT_SIZE_TO_SCALE)
        / FONT_SIZE_TO_SCALE
}

/// A calendar date picker.
///
/// A date picker widget that lets the user select a year, month and day: the days of one month
/// show at a time, a header button switches to a grid of years, and the arrows page between
/// months.
///
/// The `first_date` and `last_date` bound the selectable dates (both inclusive); the
/// [`selectable_day_predicate`](Self::selectable_day_predicate) can disable single days.
/// [`on_date_changed`](Self::on_date_changed) is called with the date the user picks.
///
/// Requires a [`Material`] ancestor, a [`MaterialLocalizations`] and a
/// [`Directionality`](waft_widgets::Directionality).
///
/// The widget takes the dates "as dates": it reads only their day, through the
/// [`calendar_delegate`](Self::calendar_delegate)'s [`CalendarDelegate::date_only`], so a time
/// of day on `initial_date` is dropped ([`effective_initial_date`](Self::effective_initial_date)
/// and its siblings).
///
/// See also:
///
///  * `showDatePicker`, which shows a dialog that contains a Material Design date picker.
///  * `showTimePicker`, which shows a dialog that contains a Material Design time picker.
pub struct CalendarDatePicker {
    pub key: Option<KeyRef>,

    /// The initially selected [`DateTime`] that the picker should display.
    ///
    /// Subsequently changing this has no effect. To change the selected date, change the
    /// [`key`](Self::key) to create a new instance of the widget.
    ///
    /// If `None`, no date is selected.
    pub initial_date: Option<DateTime>,

    /// The earliest allowable [`DateTime`] that the user can select.
    pub first_date: DateTime,

    /// The latest allowable [`DateTime`] that the user can select.
    pub last_date: DateTime,

    /// The [`DateTime`] representing today. It will be highlighted in the day grid.
    ///
    /// Defaults to [`CalendarDelegate::now`]; `None`: not given.
    pub current_date: Option<DateTime>,

    /// Called when the user selects a date in the picker.
    pub on_date_changed: ValueChanged<DateTime>,

    /// Called when the user navigates to a new month/year in the picker.
    pub on_displayed_month_changed: Option<ValueChanged<DateTime>>,

    /// The initial display of the calendar picker.
    pub initial_calendar_mode: DatePickerMode,

    /// Function to provide full control over which dates in the calendar can be selected.
    pub selectable_day_predicate: Option<SelectableDayPredicate>,

    /// Builds the widget of a day in the calendar, from the default one.
    pub day_builder: Option<CalendarDatePickerDayBuilder>,

    /// Builds the widget of a weekday header, from the default one.
    pub weekday_builder: Option<CalendarDatePickerWeekdayBuilder>,

    /// The calendar system to use.
    ///
    /// Defaults to [`GregorianCalendarDelegate`].
    pub calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl CalendarDatePicker {
    /// Creates a calendar date picker; Dart's optional named arguments are the setters.
    ///
    /// It will display a grid of days for the `initial_date`'s month, if non-`None`, or the
    /// current date's month otherwise. The day indicated by `initial_date` will be selected if
    /// it is non-`None`.
    pub fn new(
        initial_date: Option<DateTime>,
        first_date: DateTime,
        last_date: DateTime,
        on_date_changed: ValueChanged<DateTime>,
    ) -> CalendarDatePicker {
        CalendarDatePicker {
            key: None,
            initial_date,
            first_date,
            last_date,
            current_date: None,
            on_date_changed,
            on_displayed_month_changed: None,
            initial_calendar_mode: DatePickerMode::Day,
            selectable_day_predicate: None,
            day_builder: None,
            weekday_builder: None,
            calendar_delegate: Rc::new(GregorianCalendarDelegate),
        }
    }

    /// Dart's `initialDate` field, as its constructor fills it: the day of the date given.
    pub fn effective_initial_date(&self) -> Option<DateTime> {
        self.initial_date
            .map(|date| self.calendar_delegate.date_only(&date))
    }

    /// Dart's `firstDate` field, as its constructor fills it: the day of the date given.
    pub fn effective_first_date(&self) -> DateTime {
        self.calendar_delegate.date_only(&self.first_date)
    }

    /// Dart's `lastDate` field, as its constructor fills it: the day of the date given.
    pub fn effective_last_date(&self) -> DateTime {
        self.calendar_delegate.date_only(&self.last_date)
    }

    /// Dart's `currentDate` field, as its constructor fills it: the day of the date given, or
    /// of the delegate's [`now`](CalendarDelegate::now).
    pub fn effective_current_date(&self) -> DateTime {
        let current_date = self
            .current_date
            .unwrap_or_else(|| self.calendar_delegate.now());
        self.calendar_delegate.date_only(&current_date)
    }

    /// Dart `CalendarDatePicker(key:)`.
    pub fn key(mut self, key: KeyRef) -> CalendarDatePicker {
        self.key = Some(key);
        self
    }

    /// Dart `CalendarDatePicker(currentDate:)`.
    pub fn current_date(mut self, current_date: DateTime) -> CalendarDatePicker {
        self.current_date = Some(current_date);
        self
    }

    /// Dart `CalendarDatePicker(onDisplayedMonthChanged:)`.
    pub fn on_displayed_month_changed(
        mut self,
        on_displayed_month_changed: ValueChanged<DateTime>,
    ) -> CalendarDatePicker {
        self.on_displayed_month_changed = Some(on_displayed_month_changed);
        self
    }

    /// Dart `CalendarDatePicker(initialCalendarMode:)`.
    pub fn initial_calendar_mode(
        mut self,
        initial_calendar_mode: DatePickerMode,
    ) -> CalendarDatePicker {
        self.initial_calendar_mode = initial_calendar_mode;
        self
    }

    /// Dart `CalendarDatePicker(selectableDayPredicate:)`.
    pub fn selectable_day_predicate(
        mut self,
        selectable_day_predicate: SelectableDayPredicate,
    ) -> CalendarDatePicker {
        self.selectable_day_predicate = Some(selectable_day_predicate);
        self
    }

    /// Dart `CalendarDatePicker(dayBuilder:)`.
    pub fn day_builder(mut self, day_builder: CalendarDatePickerDayBuilder) -> CalendarDatePicker {
        self.day_builder = Some(day_builder);
        self
    }

    /// Dart `CalendarDatePicker(weekdayBuilder:)`.
    pub fn weekday_builder(
        mut self,
        weekday_builder: CalendarDatePickerWeekdayBuilder,
    ) -> CalendarDatePicker {
        self.weekday_builder = Some(weekday_builder);
        self
    }

    /// Dart `CalendarDatePicker(calendarDelegate:)`.
    pub fn calendar_delegate(
        mut self,
        calendar_delegate: Rc<dyn CalendarDelegate>,
    ) -> CalendarDatePicker {
        self.calendar_delegate = calendar_delegate;
        self
    }
}

impl fmt::Debug for CalendarDatePicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarDatePicker")
            .field("initial_date", &self.initial_date)
            .field("first_date", &self.first_date)
            .field("last_date", &self.last_date)
            .field("current_date", &self.current_date)
            .field("initial_calendar_mode", &self.initial_calendar_mode)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CalendarDatePicker {
    type State = CalendarDatePickerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            !self
                .effective_last_date()
                .is_before(&self.effective_first_date()),
            "lastDate {} must be on or after firstDate {}.",
            self.effective_last_date(),
            self.effective_first_date()
        );
        debug_assert!(
            self.effective_initial_date()
                .is_none_or(|date| !date.is_before(&self.effective_first_date())),
            "initialDate {:?} must be on or after firstDate {}.",
            self.effective_initial_date(),
            self.effective_first_date()
        );
        debug_assert!(
            self.effective_initial_date()
                .is_none_or(|date| !date.is_after(&self.effective_last_date())),
            "initialDate {:?} must be on or before lastDate {}.",
            self.effective_initial_date(),
            self.effective_last_date()
        );
        debug_assert!(
            self.selectable_day_predicate
                .as_ref()
                .is_none_or(|predicate| self
                    .effective_initial_date()
                    .is_none_or(|date| predicate(date))),
            "Provided initialDate {:?} must satisfy provided selectableDayPredicate.",
            self.effective_initial_date()
        );
    }

    fn create_state(&self) -> CalendarDatePickerState {
        CalendarDatePickerState {
            state: StateData::new(),
            announced_initial_date: false,
            announcement_text: String::new(),
            mode: self.initial_calendar_mode,
            current_displayed_month_date: self.calendar_delegate.get_month(1970, 1),
            selected_date: None,
            month_picker_key: GlobalKey::new(),
            year_picker_key: GlobalKey::new(),
            localizations: None,
            text_direction: None,
        }
    }
}

/// Dart's `_CalendarDatePickerState`.
pub struct CalendarDatePickerState {
    state: StateData<CalendarDatePicker>,
    announced_initial_date: bool,
    announcement_text: String,
    mode: DatePickerMode,
    current_displayed_month_date: DateTime,
    selected_date: Option<DateTime>,
    month_picker_key: GlobalKey,
    year_picker_key: GlobalKey,
    localizations: Option<Rc<dyn MaterialLocalizations>>,
    text_direction: Option<waft_painting::TextDirection>,
}

impl CalendarDatePickerState {
    fn localizations(self: Handle<Self>, app: &App) -> Rc<dyn MaterialLocalizations> {
        app.get(self)
            .localizations
            .clone()
            .expect("set in did_change_dependencies")
    }

    // Auxiliary method for handling the difference between platforms. Sending the
    // announcement itself waits for the semantics tree.
    fn announce(self: Handle<Self>, app: &mut App, message: String) {
        let context = self.context(app);
        if !MediaQuery::maybe_supports_announce_of(app, context).unwrap_or(false) {
            // If SemanticsService.sendAnnouncement is not supported,
            // we use live region to achieve the announcement effect instead.
            app.get_mut(self).announcement_text = message;
        }
    }

    fn vibrate(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let platform = Theme::of(app, context).platform;
        vibrate(app, platform);
    }

    fn handle_mode_changed(self: Handle<Self>, app: &mut App, mode: DatePickerMode) {
        self.vibrate(app);
        app.get_mut(self).mode = mode;
        if let Some(selected) = app.get(self).selected_date {
            let localizations = self.localizations(app);
            let delegate = self.widget(app).calendar_delegate.clone();
            let message = match mode {
                DatePickerMode::Day => delegate.format_month_year(&selected, &*localizations),
                DatePickerMode::Year => delegate.format_year(selected.year(), &*localizations),
            };
            self.announce(app, message);
        }
        self.set_state(app, |_| {});
    }

    fn handle_month_changed(self: Handle<Self>, app: &mut App, date: DateTime) {
        let displayed = app.get(self).current_displayed_month_date;
        if displayed.year() != date.year() || displayed.month() != date.month() {
            let widget = self.widget(app);
            let delegate = widget.calendar_delegate.clone();
            let on_displayed_month_changed = widget.on_displayed_month_changed.clone();
            let month = delegate.get_month(date.year(), date.month());
            app.get_mut(self).current_displayed_month_date = month;
            if let Some(on_displayed_month_changed) = on_displayed_month_changed {
                on_displayed_month_changed(app, month);
            }
        }
        self.set_state(app, |_| {});
    }

    fn handle_year_changed(self: Handle<Self>, app: &mut App, value: DateTime) {
        self.vibrate(app);

        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let first_date = widget.effective_first_date();
        let last_date = widget.effective_last_date();
        let on_date_changed = widget.on_date_changed.clone();
        let days_in_month = delegate.get_days_in_month(value.year(), value.month());
        let preferred_day = app
            .get(self)
            .selected_date
            .map_or(1, |date| date.day())
            .min(days_in_month);
        let mut value = delegate.get_day(value.year(), value.month(), preferred_day);

        if value.is_before(&first_date) {
            value = first_date;
        } else if value.is_after(&last_date) {
            value = last_date;
        }

        app.get_mut(self).mode = DatePickerMode::Day;
        self.handle_month_changed(app, value);

        if self.is_selectable(app, &value) {
            app.get_mut(self).selected_date = Some(value);
            on_date_changed(app, value);
        }
        self.set_state(app, |_| {});
    }

    fn handle_day_changed(self: Handle<Self>, app: &mut App, value: DateTime) {
        self.vibrate(app);
        app.get_mut(self).selected_date = Some(value);
        let on_date_changed = self.widget(app).on_date_changed.clone();
        on_date_changed(app, value);
        // The platforms that announce a selection do it through the semantics tree, which
        // is not ported.
        self.set_state(app, |_| {});
    }

    fn is_selectable(self: Handle<Self>, app: &App, date: &DateTime) -> bool {
        self.widget(app)
            .selectable_day_predicate
            .as_ref()
            .is_none_or(|predicate| predicate(*date))
    }

    fn build_picker(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let current_date = widget.effective_current_date();
        let first_date = widget.effective_first_date();
        let last_date = widget.effective_last_date();
        let selectable_day_predicate = widget.selectable_day_predicate.clone();
        let day_builder = widget.day_builder.clone();
        let weekday_builder = widget.weekday_builder.clone();
        let state = app.get(self);
        let displayed_month = state.current_displayed_month_date;
        let selected_date = state.selected_date;
        match state.mode {
            DatePickerMode::Day => {
                let month_picker_key = state.month_picker_key.clone();
                let mut month_picker = MonthPicker::new(
                    displayed_month,
                    current_date,
                    first_date,
                    last_date,
                    selected_date,
                    Rc::new(move |app, date| self.handle_day_changed(app, date)),
                    Rc::new(move |app, date| self.handle_month_changed(app, date)),
                    delegate,
                );
                month_picker.selectable_day_predicate = selectable_day_predicate;
                month_picker.day_builder = day_builder;
                month_picker.weekday_builder = weekday_builder;
                month_picker.key(month_picker_key.into()).into_widget()
            }
            DatePickerMode::Year => {
                let year_picker_key = state.year_picker_key.clone();
                Padding::new(EdgeInsetsGeometry::only(0.0, SUB_HEADER_HEIGHT, 0.0, 0.0))
                    .child(
                        YearPicker::new(
                            first_date,
                            last_date,
                            Some(displayed_month),
                            Rc::new(move |app, date| self.handle_year_changed(app, date)),
                        )
                        .key(year_picker_key.into())
                        .calendar_delegate(delegate)
                        .current_date(current_date),
                    )
                    .into_widget()
            }
        }
    }
}

impl State for CalendarDatePickerState {
    type Widget = CalendarDatePicker;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let initial_date = widget.effective_initial_date();
        let current_displayed_date = initial_date.unwrap_or(widget.effective_current_date());
        let month = delegate.get_month(
            current_displayed_date.year(),
            current_displayed_date.month(),
        );
        let state = app.get_mut(self);
        state.current_displayed_month_date = month;
        if initial_date.is_some() {
            state.selected_date = initial_date;
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(waft_widgets::debug_check_has_directionality(
            app, context, None, None, None
        ));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let text_direction = Directionality::of(app, context);
        let state = app.get_mut(self);
        state.localizations = Some(localizations.clone());
        state.text_direction = Some(text_direction);
        let widget = self.widget(app);
        let (initial_date, current_date) = (
            widget.effective_initial_date(),
            widget.effective_current_date(),
        );
        let delegate = widget.calendar_delegate.clone();
        if !app.get(self).announced_initial_date && initial_date.is_some() {
            let selected = app
                .get(self)
                .selected_date
                .expect("the initial date is selected");
            app.get_mut(self).announced_initial_date = true;
            let is_today = delegate.is_same_day(Some(&current_date), Some(&selected));
            let semantic_label_suffix = if is_today {
                format!(", {}", localizations.current_date_label())
            } else {
                String::new()
            };
            self.announce(
                app,
                format!(
                    "{}{}",
                    localizations.format_full_date(&selected),
                    semantic_label_suffix
                ),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(waft_widgets::debug_check_has_directionality(
            app, context, None, None, None
        ));
        let text_scale_factor = clamped_text_scale_factor(app, context, MAX_TEXT_SCALE_FACTOR);

        // Conform to M3 spec in portrait mode (landscape mode is not specified).
        let orientation = MediaQuery::orientation_of(app, context);
        let max_day_picker_height =
            if Theme::of(app, context).use_material3 && orientation == Orientation::Portrait {
                MAX_DAY_PICKER_HEIGHT_M3
            } else {
                MAX_DAY_PICKER_HEIGHT_M2
            };

        // Scale the height of the picker area up with larger text. The size of the
        // picker has room for larger text, up until a scale factor of 1.3. After
        // after which, we increase the height to add room for content to continue
        // to scale the text size.
        let scaled_max_day_picker_height = if text_scale_factor > 1.3 {
            max_day_picker_height
                + (f64::from(MAX_DAY_PICKER_ROW_COUNT + 1) * ((text_scale_factor - 1.0) * 8.0))
        } else {
            max_day_picker_height
        };
        let picker = SizedBox::new()
            .height(SUB_HEADER_HEIGHT + scaled_max_day_picker_height)
            .child(self.build_picker(app))
            .into_widget();
        let supports_announce =
            MediaQuery::maybe_supports_announce_of(app, context).unwrap_or(false);
        let picker = if supports_announce {
            picker
        } else {
            Semantics::new()
                .container(true)
                .live_region(true)
                .accessibility_focus_block_type(
                    waft_rendering::AccessibilityFocusBlockType::BlockNode,
                )
                .label(app.get(self).announcement_text.clone())
                .child(picker)
                .into_widget()
        };

        let localizations = self.localizations(app);
        let delegate = self.widget(app).calendar_delegate.clone();
        let mode = app.get(self).mode;
        let title = delegate
            .format_month_year(&app.get(self).current_displayed_month_date, &*localizations);
        Stack::new()
            .children([
                picker,
                // Put the mode toggle button on top so that it won't be covered up by the
                // MonthPicker
                MediaQuery::with_clamped_text_scaling(
                    None,
                    0.0,
                    MODE_TOGGLE_BUTTON_MAX_SCALE_FACTOR,
                    DatePickerModeToggleButton::new(
                        mode,
                        title,
                        Listener::new(move |app: &mut App| {
                            let mode = match app.get(self).mode {
                                DatePickerMode::Day => DatePickerMode::Year,
                                DatePickerMode::Year => DatePickerMode::Day,
                            };
                            self.handle_mode_changed(app, mode)
                        }),
                    ),
                ),
            ])
            .into_widget()
    }
}

impl fmt::Debug for CalendarDatePickerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarDatePickerState")
            .field("mode", &self.mode)
            .field("selected_date", &self.selected_date)
            .finish_non_exhaustive()
    }
}

/// Dart's `_DatePickerModeToggleButton`.
struct DatePickerModeToggleButton {
    mode: DatePickerMode,
    title: String,
    on_title_pressed: Listener,
}

impl DatePickerModeToggleButton {
    fn new(
        mode: DatePickerMode,
        title: String,
        on_title_pressed: Listener,
    ) -> DatePickerModeToggleButton {
        DatePickerModeToggleButton {
            mode,
            title,
            on_title_pressed,
        }
    }
}

impl fmt::Debug for DatePickerModeToggleButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatePickerModeToggleButton")
            .field("mode", &self.mode)
            .field("title", &self.title)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for DatePickerModeToggleButton {
    type State = DatePickerModeToggleButtonState;

    fn create_state(&self) -> DatePickerModeToggleButtonState {
        DatePickerModeToggleButtonState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
        }
    }
}

/// Dart's `_DatePickerModeToggleButtonState`.
struct DatePickerModeToggleButtonState {
    state: StateData<DatePickerModeToggleButton>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Created in `init_state`, because a state is created without the [`App`].
    controller: Option<Ref<AnimationController>>,
}

impl DatePickerModeToggleButtonState {
    fn controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .controller
            .clone()
            .expect("created in init_state")
    }
}

impl SingleTickerProviderStateMixin for DatePickerModeToggleButtonState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for DatePickerModeToggleButtonState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for DatePickerModeToggleButtonState {
    type Widget = DatePickerModeToggleButton;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let value = if self.widget(app).mode == DatePickerMode::Year {
            0.5
        } else {
            0.0
        };
        let controller = AnimationController::new(
            app,
            Some(value),
            Some(Duration::from_millis(200)),
            None,
            0.0,
            0.5,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &DatePickerModeToggleButton,
    ) {
        let mode = self.widget(app).mode;
        if old_widget.mode == mode {
            return;
        }

        let controller = self.controller(app);
        if mode == DatePickerMode::Year {
            controller.forward(app, None);
        } else {
            controller.reverse(app, None);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let button_text_style = date_picker_theme
            .toggle_button_text_style
            .clone()
            .or_else(|| defaults.toggle_button_text_style(app));
        let sub_header_foreground_color = date_picker_theme
            .sub_header_foreground_color
            .or_else(|| defaults.sub_header_foreground_color(app));
        let button_text_color = date_picker_theme
            .toggle_button_text_style
            .as_ref()
            .and_then(|style| style.color.as_ref().map(|color| color.color()))
            .or(date_picker_theme.sub_header_foreground_color)
            .or_else(|| {
                defaults
                    .toggle_button_text_style(app)
                    .and_then(|style| style.color.as_ref().map(|color| color.color()))
            });

        let widget = self.widget(app);
        let mode = widget.mode;
        let title = widget.title.clone();
        let on_title_pressed = widget.on_title_pressed.clone();
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let controller = self.controller(app);

        let mut title_text = Text::new(title).overflow(TextOverflow::Ellipsis);
        if let Some(style) = apply_color(button_text_style, button_text_color) {
            title_text = title_text.style(style);
        }
        let mut arrow = Icon::new(Some(Icons::arrow_drop_down()));
        if let Some(color) = sub_header_foreground_color {
            arrow = arrow.color(color);
        }
        let mut children = vec![
            Flexible::new(
                Semantics::new()
                    .label(localizations.select_year_semantics_label())
                    .expanded(mode == DatePickerMode::Year)
                    .button(true)
                    .container(true)
                    .child(
                        SizedBox::new().height(SUB_HEADER_HEIGHT).child(
                            InkWell::new().on_tap(on_title_pressed).child(
                                Padding::new(EdgeInsetsGeometry::symmetric(0.0, 8.0)).child(
                                    Row::new().children([
                                        Flexible::new(title_text).into_widget(),
                                        RotationTransition::new(controller.erase())
                                            .child(arrow)
                                            .into_widget(),
                                    ]),
                                ),
                            ),
                        ),
                    ),
            )
            .into_widget(),
        ];
        if mode == DatePickerMode::Day {
            // Give space for the prev/next month buttons that are underneath this row
            children.push(SizedBox::new().width(MONTH_NAV_BUTTONS_WIDTH).into_widget());
        }
        SizedBox::new()
            .height(SUB_HEADER_HEIGHT)
            .child(
                Padding::new(EdgeInsetsDirectional::only(16.0, 0.0, 4.0, 0.0).into())
                    .child(Row::new().children(children)),
            )
            .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.handle().dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }
}

impl fmt::Debug for DatePickerModeToggleButtonState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatePickerModeToggleButtonState")
            .finish_non_exhaustive()
    }
}

/// Dart's `_FocusedDate`: the day the day grid has focus on, for the [`DayPicker`] that shows
/// its month to take focus.
#[derive(Debug)]
struct FocusedDate {
    child: WidgetRef,
    calendar_delegate: Rc<dyn CalendarDelegate>,
    date: Option<DateTime>,
}

impl FocusedDate {
    fn maybe_of(app: &mut App, context: BuildContext) -> Option<DateTime> {
        context
            .depend_on_inherited_widget_of_exact_type::<FocusedDate>(app)
            .and_then(|focused_date| focused_date.date)
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
    }
}

/// Dart's `_MonthPicker`: the pages of months, with the buttons that go between them.
struct MonthPicker {
    key: Option<KeyRef>,
    initial_month: DateTime,
    current_date: DateTime,
    first_date: DateTime,
    last_date: DateTime,
    selected_date: Option<DateTime>,
    on_changed: ValueChanged<DateTime>,
    on_displayed_month_changed: ValueChanged<DateTime>,
    selectable_day_predicate: Option<SelectableDayPredicate>,
    day_builder: Option<CalendarDatePickerDayBuilder>,
    weekday_builder: Option<CalendarDatePickerWeekdayBuilder>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl MonthPicker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        initial_month: DateTime,
        current_date: DateTime,
        first_date: DateTime,
        last_date: DateTime,
        selected_date: Option<DateTime>,
        on_changed: ValueChanged<DateTime>,
        on_displayed_month_changed: ValueChanged<DateTime>,
        calendar_delegate: Rc<dyn CalendarDelegate>,
    ) -> MonthPicker {
        MonthPicker {
            key: None,
            initial_month,
            current_date,
            first_date,
            last_date,
            selected_date,
            on_changed,
            on_displayed_month_changed,
            selectable_day_predicate: None,
            day_builder: None,
            weekday_builder: None,
            calendar_delegate,
        }
    }

    fn key(mut self, key: KeyRef) -> MonthPicker {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for MonthPicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonthPicker")
            .field("initial_month", &self.initial_month)
            .field("selected_date", &self.selected_date)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MonthPicker {
    type State = MonthPickerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(!self.first_date.is_after(&self.last_date));
        debug_assert!(
            self.selected_date
                .is_none_or(|date| !date.is_before(&self.first_date))
        );
        debug_assert!(
            self.selected_date
                .is_none_or(|date| !date.is_after(&self.last_date))
        );
    }

    fn create_state(&self) -> MonthPickerState {
        MonthPickerState {
            state: StateData::new(),
            page_view_key: GlobalKey::new(),
            announcement_text: String::new(),
            current_month: self.initial_month,
            page_controller: None,
            localizations: None,
            shortcut_map: Vec::new(),
            action_map: HashMap::new(),
            day_grid_focus: None,
            focused_day: None,
        }
    }
}

/// Dart's `_MonthPickerState`.
struct MonthPickerState {
    state: StateData<MonthPicker>,
    page_view_key: GlobalKey,
    announcement_text: String,
    current_month: DateTime,
    /// Created in `init_state`, because a state is created without the [`App`].
    page_controller: Option<Ref<PageController>>,
    localizations: Option<Rc<dyn MaterialLocalizations>>,
    shortcut_map: ShortcutMap,
    action_map: HashMap<TypeId, ActionRef>,
    day_grid_focus: Option<Ref<FocusNode>>,
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

/// Dart's `_directionOffset`.
fn direction_offset(direction: TraversalDirection) -> i32 {
    match direction {
        TraversalDirection::Up => -DateTime::DAYS_PER_WEEK,
        TraversalDirection::Right => 1,
        TraversalDirection::Down => DateTime::DAYS_PER_WEEK,
        TraversalDirection::Left => -1,
    }
}

impl MonthPickerState {
    fn page_controller(self: Handle<Self>, app: &App) -> Handle<PageController> {
        app.get(self)
            .page_controller
            .as_ref()
            .expect("created in init_state")
            .handle()
    }

    fn day_grid_focus(self: Handle<Self>, app: &App) -> Ref<FocusNode> {
        app.get(self)
            .day_grid_focus
            .clone()
            .expect("created in init_state")
    }

    fn localizations(self: Handle<Self>, app: &App) -> Rc<dyn MaterialLocalizations> {
        app.get(self)
            .localizations
            .clone()
            .expect("set in did_change_dependencies")
    }

    fn handle_date_selected(self: Handle<Self>, app: &mut App, selected_date: DateTime) {
        app.get_mut(self).focused_day = Some(selected_date);
        let on_changed = self.widget(app).on_changed.clone();
        on_changed(app, selected_date);
    }

    // Auxiliary method for handling the difference between platforms. Sending the
    // announcement itself waits for the semantics tree.
    fn announce(self: Handle<Self>, app: &mut App, message: String) {
        let context = self.context(app);
        if !MediaQuery::maybe_supports_announce_of(app, context).unwrap_or(false) {
            // If SemanticsService.sendAnnouncement is not supported,
            // we use live region to achieve the announcement effect instead.
            app.get_mut(self).announcement_text = message;
        }
    }

    fn handle_month_page_changed(self: Handle<Self>, app: &mut App, month_page: i32) {
        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let on_displayed_month_changed = widget.on_displayed_month_changed.clone();
        let month_date = delegate.add_months_to_month_date(&widget.first_date, month_page);
        let current_month = app.get(self).current_month;
        if !delegate.is_same_month(Some(&current_month), Some(&month_date)) {
            let current_month = delegate.get_month(month_date.year(), month_date.month());
            app.get_mut(self).current_month = current_month;
            on_displayed_month_changed(app, current_month);
            if let Some(focused_day) = app.get(self).focused_day
                && !delegate.is_same_month(Some(&focused_day), Some(&current_month))
            {
                // We have navigated to a new month with the grid focused, but the
                // focused day is not in this month. Choose a new one trying to keep
                // the same day of the month.
                let focusable =
                    self.focusable_day_for_month(app, &current_month, focused_day.day());
                app.get_mut(self).focused_day = focusable;
            }
            let localizations = self.localizations(app);
            self.announce(
                app,
                delegate.format_month_year(&current_month, &*localizations),
            );
        }
        self.set_state(app, |_| {});
    }

    fn focusable_day_for_month(
        self: Handle<Self>,
        app: &App,
        month: &DateTime,
        preferred_day: i32,
    ) -> Option<DateTime> {
        let delegate = &self.widget(app).calendar_delegate;
        let days_in_month = delegate.get_days_in_month(month.year(), month.month());

        // Can we use the preferred day in this month?
        if preferred_day <= days_in_month {
            let new_focus = delegate.get_day(month.year(), month.month(), preferred_day);
            if self.is_selectable(app, &new_focus) {
                return Some(new_focus);
            }
        }

        // Start at the 1st and take the first selectable date.
        for day in 1..=days_in_month {
            let new_focus = delegate.get_day(month.year(), month.month(), day);
            if self.is_selectable(app, &new_focus) {
                return Some(new_focus);
            }
        }
        None
    }

    fn handle_next_month(self: Handle<Self>, app: &mut App) {
        if !self.is_displaying_last_month(app) {
            self.page_controller(app)
                .next_page(app, MONTH_SCROLL_DURATION, Curves::ease());
        }
    }

    fn handle_previous_month(self: Handle<Self>, app: &mut App) {
        if !self.is_displaying_first_month(app) {
            self.page_controller(app)
                .previous_page(app, MONTH_SCROLL_DURATION, Curves::ease());
        }
    }

    fn show_month(self: Handle<Self>, app: &mut App, month: &DateTime, jump: bool) {
        let widget = self.widget(app);
        let month_page = widget
            .calendar_delegate
            .month_delta(&widget.first_date, month);
        if jump {
            self.page_controller(app).jump_to_page(app, month_page);
        } else {
            self.page_controller(app).animate_to_page(
                app,
                month_page,
                MONTH_SCROLL_DURATION,
                Curves::ease(),
            );
        }
    }

    fn is_displaying_first_month(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        !app.get(self).current_month.is_after(
            &widget
                .calendar_delegate
                .get_month(widget.first_date.year(), widget.first_date.month()),
        )
    }

    fn is_displaying_last_month(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        !app.get(self).current_month.is_before(
            &widget
                .calendar_delegate
                .get_month(widget.last_date.year(), widget.last_date.month()),
        )
    }

    fn handle_grid_focus_change(self: Handle<Self>, app: &mut App, focused: bool) {
        if focused && app.get(self).focused_day.is_none() {
            let widget = self.widget(app);
            let delegate = widget.calendar_delegate.clone();
            let selected_date = widget.selected_date;
            let current_date = widget.current_date;
            let current_month = app.get(self).current_month;
            let focused_day =
                if delegate.is_same_month(selected_date.as_ref(), Some(&current_month)) {
                    selected_date
                } else if delegate.is_same_month(Some(&current_date), Some(&current_month)) {
                    self.focusable_day_for_month(app, &current_month, current_date.day())
                } else {
                    self.focusable_day_for_month(app, &current_month, 1)
                };
            app.get_mut(self).focused_day = focused_day;
        }
        self.set_state(app, |_| {});
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
        if let Some(next_date) = self.next_date_in_direction(app, &focused_day, direction) {
            app.get_mut(self).focused_day = Some(next_date);
            let delegate = self.widget(app).calendar_delegate.clone();
            let current_month = app.get(self).current_month;
            if !delegate.is_same_month(Some(&next_date), Some(&current_month)) {
                self.show_month(app, &next_date, false);
            }
        }
        self.set_state(app, |_| {});
    }

    fn day_direction_offset(
        traversal_direction: TraversalDirection,
        text_direction: waft_painting::TextDirection,
    ) -> i32 {
        let mut traversal_direction = traversal_direction;
        // Swap left and right if the text direction if RTL
        if text_direction == waft_painting::TextDirection::Rtl {
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
        let delegate = widget.calendar_delegate.clone();
        let (first_date, last_date) = (widget.first_date, widget.last_date);
        let offset = Self::day_direction_offset(direction, text_direction);
        let mut next_date = delegate.add_days_to_date(date, offset);
        while !next_date.is_before(&first_date) && !next_date.is_after(&last_date) {
            if self.is_selectable(app, &next_date) {
                return Some(next_date);
            }
            next_date = delegate.add_days_to_date(&next_date, offset);
        }
        None
    }

    fn is_selectable(self: Handle<Self>, app: &App, date: &DateTime) -> bool {
        self.widget(app)
            .selectable_day_predicate
            .as_ref()
            .is_none_or(|predicate| predicate(*date))
    }

    fn build_items(self: Handle<Self>, app: &mut App, index: i32) -> WidgetRef {
        let widget = self.widget(app);
        let month = widget
            .calendar_delegate
            .add_months_to_month_date(&widget.first_date, index);
        let mut day_picker = DayPicker::new(
            widget.current_date,
            month,
            widget.first_date,
            widget.last_date,
            widget.selected_date,
            Rc::new(move |app, date| self.handle_date_selected(app, date)),
            widget.calendar_delegate.clone(),
        );
        day_picker.selectable_day_predicate = widget.selectable_day_predicate.clone();
        day_picker.day_builder = widget.day_builder.clone();
        day_picker.weekday_builder = widget.weekday_builder.clone();
        day_picker
            .key(waft_foundation::ValueKey::new(month).into())
            .into_widget()
    }
}

impl State for MonthPickerState {
    type Widget = MonthPicker;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let current_month = widget.initial_month;
        let initial_page = widget
            .calendar_delegate
            .month_delta(&widget.first_date, &current_month);
        app.get_mut(self).current_month = current_month;
        let page_controller = PageController::new(app, initial_page, true, 1.0, None, None);
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
            .set_debug_label(app, Some("Day Grid".to_owned()));
        let state = app.get_mut(self);
        state.page_controller = Some(page_controller);
        state.shortcut_map = shortcut_map;
        state.action_map = action_map;
        state.day_grid_focus = Some(day_grid_focus);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        app.get_mut(self).localizations = Some(localizations);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.page_controller(app).dispose(app);
        self.day_grid_focus(app).handle().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let sub_header_foreground_color = DatePickerTheme::of(app, context)
            .sub_header_foreground_color
            .or_else(|| DatePickerTheme::defaults(app, context).sub_header_foreground_color(app));

        let supports_announce =
            MediaQuery::maybe_supports_announce_of(app, context).unwrap_or(false);
        let localizations = self.localizations(app);
        let is_displaying_first_month = self.is_displaying_first_month(app);
        let is_displaying_last_month = self.is_displaying_last_month(app);

        let mut previous_icon = Icon::new(Some(Icons::chevron_left()));
        if is_displaying_first_month {
            previous_icon = previous_icon.semantic_label(localizations.previous_month_tooltip());
        }
        let mut previous_button = IconButton::new(
            (!is_displaying_first_month)
                .then(|| Listener::handle_method(self, Self::handle_previous_month)),
            previous_icon,
        );
        if let Some(color) = sub_header_foreground_color {
            previous_button = previous_button.color(color);
        }
        if !is_displaying_first_month {
            previous_button = previous_button.tooltip(localizations.previous_month_tooltip());
        }
        let mut next_icon = Icon::new(Some(Icons::chevron_right()));
        if is_displaying_last_month {
            next_icon = next_icon.semantic_label(localizations.next_month_tooltip());
        }
        let mut next_button = IconButton::new(
            (!is_displaying_last_month)
                .then(|| Listener::handle_method(self, Self::handle_next_month)),
            next_icon,
        );
        if let Some(color) = sub_header_foreground_color {
            next_button = next_button.color(color);
        }
        if !is_displaying_last_month {
            next_button = next_button.tooltip(localizations.next_month_tooltip());
        }

        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let item_count = delegate.month_delta(&widget.first_date, &widget.last_date) + 1;
        let state = app.get(self);
        let page_controller = state
            .page_controller
            .clone()
            .expect("created in init_state");
        let day_grid_focus = state.day_grid_focus.clone().expect("created in init_state");
        let page_view_key = state.page_view_key.clone();
        let grid_has_focus = day_grid_focus.handle().has_focus(app);
        let focused_date = if grid_has_focus {
            app.get(self).focused_day
        } else {
            None
        };
        let announcement_text = app.get(self).announcement_text.clone();
        let shortcuts = app.get(self).shortcut_map.clone();
        let actions = app.get(self).action_map.clone();

        let mut semantics = Semantics::new()
            .container(true)
            .explicit_child_nodes(true)
            .live_region(!supports_announce)
            .accessibility_focus_block_type(if supports_announce {
                waft_rendering::AccessibilityFocusBlockType::None
            } else {
                waft_rendering::AccessibilityFocusBlockType::BlockNode
            });
        if !supports_announce {
            semantics = semantics.label(announcement_text);
        }
        semantics
            .child(
                Column::new().children([
                    SizedBox::new()
                        .height(SUB_HEADER_HEIGHT)
                        .child(
                            Padding::new(EdgeInsetsDirectional::only(16.0, 0.0, 4.0, 0.0).into())
                                .child(Row::new().children([
                                    Spacer::new().into_widget(),
                                    previous_button.into_widget(),
                                    next_button.into_widget(),
                                ])),
                        )
                        .into_widget(),
                    Expanded::new(
                        FocusableActionDetector::new(FocusedDate {
                            calendar_delegate: delegate,
                            date: focused_date,
                            // Wrap the PageView with `Material`, so when its child paints on
                            // materials the content won't go out of boundary during page
                            // transition.
                            child: Material::new()
                                .r#type(MaterialType::Transparency)
                                .child(
                                    PageView::builder(move |app, _context, index| {
                                        Some(self.build_items(app, index))
                                    })
                                    .item_count(item_count)
                                    .build()
                                    .key(page_view_key.into())
                                    .controller(page_controller)
                                    .on_page_changed(
                                        move |app: &mut App, page: i32| {
                                            self.handle_month_page_changed(app, page)
                                        },
                                    ),
                                )
                                .into_widget(),
                        })
                        .shortcuts(shortcuts)
                        .actions(actions)
                        .focus_node(day_grid_focus.into_erased())
                        .on_focus_change(move |app, focused| {
                            self.handle_grid_focus_change(app, focused)
                        }),
                    )
                    .into_widget(),
                ]),
            )
            .into_widget()
    }
}

impl fmt::Debug for MonthPickerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonthPickerState")
            .field("current_month", &self.current_month)
            .field("focused_day", &self.focused_day)
            .finish_non_exhaustive()
    }
}

/// Dart's `_DayPicker`: the grid of one month's days under the weekday headers.
struct DayPicker {
    key: Option<KeyRef>,
    selected_date: Option<DateTime>,
    current_date: DateTime,
    on_changed: ValueChanged<DateTime>,
    first_date: DateTime,
    last_date: DateTime,
    displayed_month: DateTime,
    selectable_day_predicate: Option<SelectableDayPredicate>,
    day_builder: Option<CalendarDatePickerDayBuilder>,
    weekday_builder: Option<CalendarDatePickerWeekdayBuilder>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl DayPicker {
    fn new(
        current_date: DateTime,
        displayed_month: DateTime,
        first_date: DateTime,
        last_date: DateTime,
        selected_date: Option<DateTime>,
        on_changed: ValueChanged<DateTime>,
        calendar_delegate: Rc<dyn CalendarDelegate>,
    ) -> DayPicker {
        DayPicker {
            key: None,
            selected_date,
            current_date,
            on_changed,
            first_date,
            last_date,
            displayed_month,
            selectable_day_predicate: None,
            day_builder: None,
            weekday_builder: None,
            calendar_delegate,
        }
    }

    fn key(mut self, key: KeyRef) -> DayPicker {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for DayPicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DayPicker")
            .field("displayed_month", &self.displayed_month)
            .field("selected_date", &self.selected_date)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for DayPicker {
    type State = DayPickerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(!self.first_date.is_after(&self.last_date));
        debug_assert!(
            self.selected_date
                .is_none_or(|date| !date.is_before(&self.first_date))
        );
        debug_assert!(
            self.selected_date
                .is_none_or(|date| !date.is_after(&self.last_date))
        );
    }

    fn create_state(&self) -> DayPickerState {
        DayPickerState {
            state: StateData::new(),
            day_focus_nodes: Vec::new(),
        }
    }
}

/// Dart's `_DayPickerState`.
struct DayPickerState {
    state: StateData<DayPicker>,
    /// Created in `init_state`, because a state is created without the [`App`].
    day_focus_nodes: Vec<Ref<FocusNode>>,
}

impl DayPickerState {
    fn day_headers(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        header_style: Option<&TextStyle>,
        localizations: &dyn MaterialLocalizations,
    ) -> Vec<WidgetRef> {
        let weekday_builder = self.widget(app).weekday_builder.clone();
        let narrow_weekdays = localizations.narrow_weekdays();
        let mut result = Vec::new();
        let mut i = localizations.first_day_of_week_index();
        while result.len() < DateTime::DAYS_PER_WEEK as usize {
            let weekday = narrow_weekdays[i as usize].clone();
            if let Some(weekday_builder) = &weekday_builder {
                let date_time_weekday = if i == 0 { DateTime::SUNDAY } else { i };
                let weekday_widget = weekday_builder(
                    app,
                    context,
                    CalendarDatePickerWeekdayDetails {
                        weekday: date_time_weekday,
                        child: ExcludeSemantics::new()
                            .child(Text::new(weekday))
                            .into_widget(),
                    },
                );
                let mut merge = DefaultTextStyle::merge(Center::new().child(weekday_widget));
                if let Some(header_style) = header_style {
                    merge = merge.style(header_style.clone());
                }
                result.push(merge.into_widget());
            } else {
                let mut text = Text::new(weekday);
                if let Some(header_style) = header_style {
                    text = text.style(header_style.clone());
                }
                result.push(
                    ExcludeSemantics::new()
                        .child(Center::new().child(text))
                        .into_widget(),
                );
            }
            i = (i + 1) % DateTime::DAYS_PER_WEEK;
        }
        result
    }
}

impl State for DayPickerState {
    type Widget = DayPicker;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
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
        let focused_date = FocusedDate::maybe_of(app, context);
        let widget = self.widget(app);
        if let Some(focused_date) = focused_date
            && widget
                .calendar_delegate
                .is_same_month(Some(&widget.displayed_month), Some(&focused_date))
        {
            let node = app.get(self).day_focus_nodes[(focused_date.day() - 1) as usize].handle();
            node.request_focus(app, None);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let nodes = std::mem::take(&mut app.get_mut(self).day_focus_nodes);
        for node in nodes {
            node.handle().dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let weekday_style = date_picker_theme
            .weekday_style
            .clone()
            .or_else(|| defaults.weekday_style(app));

        let orientation = MediaQuery::orientation_of(app, context);
        let is_landscape_orientation = orientation == Orientation::Landscape;
        let text_scale_factor = clamped_text_scale_factor(app, context, 3.0);
        let use_material3 = Theme::of(app, context).use_material3;

        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let year = widget.displayed_month.year();
        let month = widget.displayed_month.month();
        let days_in_month = delegate.get_days_in_month(year, month);
        let day_offset = delegate.first_day_offset(year, month, &*localizations);
        let (first_date, last_date) = (widget.first_date, widget.last_date);
        let selected_date = widget.selected_date;
        let current_date = widget.current_date;
        let selectable_day_predicate = widget.selectable_day_predicate.clone();
        let on_changed = widget.on_changed.clone();
        let day_builder = widget.day_builder.clone();

        let mut day_items = self.day_headers(app, context, weekday_style.as_ref(), &*localizations);
        // 1-based day of month, e.g. 1-31 for January, and 1-29 for February on
        // a leap year.
        let mut day = -day_offset;
        while day < days_in_month {
            day += 1;
            if day < 1 {
                day_items.push(SizedBox::shrink().into_widget());
            } else {
                let day_to_build = delegate.get_day(year, month, day);
                let is_disabled = day_to_build.is_after(&last_date)
                    || day_to_build.is_before(&first_date)
                    || selectable_day_predicate
                        .as_ref()
                        .is_some_and(|predicate| !predicate(day_to_build));
                let is_selected_day =
                    delegate.is_same_day(selected_date.as_ref(), Some(&day_to_build));
                let is_today = delegate.is_same_day(Some(&current_date), Some(&day_to_build));

                let focus_node = app.get(self).day_focus_nodes[(day - 1) as usize].clone();
                let mut day_widget = Day::new(
                    day_to_build,
                    is_disabled,
                    is_selected_day,
                    is_today,
                    on_changed.clone(),
                    focus_node,
                    delegate.clone(),
                );
                day_widget.day_builder = day_builder.clone();
                day_items.push(
                    day_widget
                        .key(waft_foundation::ValueKey::new(day_to_build).into())
                        .into_widget(),
                );
            }
        }

        let month_picker_horizontal_padding = if use_material3 && !is_landscape_orientation {
            MONTH_PICKER_HORIZONTAL_PADDING_PORTRAIT_M3
        } else {
            MONTH_PICKER_HORIZONTAL_PADDING_OTHER
        };
        Padding::new(EdgeInsetsGeometry::symmetric(
            0.0,
            month_picker_horizontal_padding,
        ))
        .child(MediaQuery::with_clamped_text_scaling(
            None,
            0.0,
            if is_landscape_orientation {
                DAY_PICKER_GRID_LANDSCAPE_MAX_SCALE_FACTOR
            } else {
                DAY_PICKER_GRID_PORTRAIT_MAX_SCALE_FACTOR
            },
            GridView::custom(
                Rc::new(DayPickerGridDelegate {
                    text_scale_factor,
                    use_material3,
                    orientation,
                }),
                SliverChildListDelegate::new(day_items)
                    .add_repaint_boundaries(false)
                    .into_delegate(),
            )
            .physics(Rc::new(waft_widgets::ClampingScrollPhysics::new())),
        ))
        .into_widget()
    }
}

impl fmt::Debug for DayPickerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DayPickerState").finish_non_exhaustive()
    }
}

/// Dart's `_Day`: one day of the grid.
struct Day {
    key: Option<KeyRef>,
    day: DateTime,
    is_disabled: bool,
    is_selected_day: bool,
    is_today: bool,
    on_changed: ValueChanged<DateTime>,
    focus_node: Ref<FocusNode>,
    calendar_delegate: Rc<dyn CalendarDelegate>,
    day_builder: Option<CalendarDatePickerDayBuilder>,
}

impl Day {
    fn new(
        day: DateTime,
        is_disabled: bool,
        is_selected_day: bool,
        is_today: bool,
        on_changed: ValueChanged<DateTime>,
        focus_node: Ref<FocusNode>,
        calendar_delegate: Rc<dyn CalendarDelegate>,
    ) -> Day {
        Day {
            key: None,
            day,
            is_disabled,
            is_selected_day,
            is_today,
            on_changed,
            focus_node,
            calendar_delegate,
            day_builder: None,
        }
    }

    fn key(mut self, key: KeyRef) -> Day {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Day")
            .field("day", &self.day)
            .field("is_disabled", &self.is_disabled)
            .field("is_selected_day", &self.is_selected_day)
            .field("is_today", &self.is_today)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Day {
    type State = DayState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DayState {
        DayState {
            state: StateData::new(),
            states_controller: None,
        }
    }
}

/// Dart's `_DayState`.
struct DayState {
    state: StateData<Day>,
    /// Created in `init_state`, because a state is created without the [`App`].
    states_controller: Option<Ref<WidgetStatesController>>,
}

/// Dart's `effectiveValue(getProperty)` for a state-resolved color: the theme's own property
/// first, the defaults' when it is null or resolves to null.
fn resolve_color(
    theme: Option<&waft_widgets::WidgetStatePropertyRef<Option<Color>>>,
    defaults: Option<waft_widgets::WidgetStatePropertyRef<Option<Color>>>,
    states: &WidgetStates,
) -> Option<Color> {
    theme
        .and_then(|property| property.resolve(states))
        .or_else(|| defaults.and_then(|property| property.resolve(states)))
}

/// Dart's `effectiveValue(getProperty)` for a state-resolved shape.
fn resolve_shape(
    theme: Option<&waft_widgets::WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
    defaults: Option<waft_widgets::WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
    states: &WidgetStates,
) -> Option<OutlinedBorderRef> {
    theme
        .and_then(|property| property.resolve(states))
        .or_else(|| defaults.and_then(|property| property.resolve(states)))
}

/// Dart's `datePickerTheme.todayBorder` with its color replaced when it has none of its own:
/// the color of the text.
fn today_border_side(
    theme_border: Option<BorderSide>,
    default_border: Option<BorderSide>,
    text_color: Option<Color>,
) -> Option<BorderSide> {
    let has_custom_border_color = theme_border.is_some_and(|border| border.color.a != 0.0);
    if has_custom_border_color {
        theme_border
    } else {
        theme_border.or(default_border).map(|border| BorderSide {
            color: text_color.unwrap_or(border.color),
            ..border
        })
    }
}

impl DayState {
    fn states_controller(self: Handle<Self>, app: &App) -> Ref<WidgetStatesController> {
        app.get(self)
            .states_controller
            .clone()
            .expect("created in init_state")
    }
}

impl State for DayState {
    type Widget = Day;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = WidgetStatesController::new(app, None);
        app.get_mut(self).states_controller = Some(controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.states_controller(app).handle().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let defaults = DatePickerTheme::defaults(app, context);
        let date_picker_theme = DatePickerTheme::of(app, context);
        let day_style = date_picker_theme
            .day_style
            .clone()
            .or_else(|| defaults.day_style(app));

        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let widget = self.widget(app);
        let is_today = widget.is_today;
        let is_disabled = widget.is_disabled;
        let is_selected_day = widget.is_selected_day;
        let day = widget.day;
        let delegate = widget.calendar_delegate.clone();
        let day_builder = widget.day_builder.clone();
        let on_changed = widget.on_changed.clone();
        let focus_node = widget.focus_node.clone();
        let semantic_label_suffix = if is_today {
            format!(", {}", localizations.current_date_label())
        } else {
            String::new()
        };

        let mut states = WidgetStates::new();
        if is_disabled {
            states.insert(WidgetState::Disabled);
        }
        if is_selected_day {
            states.insert(WidgetState::Selected);
        }

        let states_controller = self.states_controller(app);
        // Preserve interaction states managed by InkResponse across rebuilds.
        let interaction_states: WidgetStates = if is_disabled {
            WidgetStates::new()
        } else {
            app.get(states_controller.handle())
                .value()
                .difference(&WidgetStates::from([
                    WidgetState::Disabled,
                    WidgetState::Selected,
                ]))
                .copied()
                .collect()
        };
        states_controller
            .handle()
            .set_value(app, states.union(&interaction_states).copied().collect());

        let day_foreground_color = if is_today {
            resolve_color(
                date_picker_theme.today_foreground_color.as_ref(),
                defaults.today_foreground_color(app),
                &states,
            )
        } else {
            resolve_color(
                date_picker_theme.day_foreground_color.as_ref(),
                defaults.day_foreground_color(app),
                &states,
            )
        };
        let day_background_color = if is_today {
            resolve_color(
                date_picker_theme.today_background_color.as_ref(),
                defaults.today_background_color(app),
                &states,
            )
        } else {
            resolve_color(
                date_picker_theme.day_background_color.as_ref(),
                defaults.day_background_color(app),
                &states,
            )
        };
        let theme_day_overlay_color = date_picker_theme.day_overlay_color.clone();
        let default_day_overlay_color = defaults.day_overlay_color(app);
        let day_overlay_color =
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                resolve_color(
                    theme_day_overlay_color.as_ref(),
                    default_day_overlay_color.clone(),
                    states,
                )
            });
        let day_shape = resolve_shape(
            date_picker_theme.day_shape.as_ref(),
            defaults.day_shape(app),
            &states,
        )
        .expect("the defaults have a day shape");
        let today_border_side = today_border_side(
            date_picker_theme.today_border,
            defaults.today_border(app),
            day_foreground_color,
        );
        let day_decoration_shape: ShapeBorderRef = match &today_border_side {
            Some(side) if is_today => day_shape.copy_with(Some(*side)).into(),
            _ => day_shape.clone().into(),
        };
        let mut decoration = ShapeDecoration::new(day_decoration_shape);
        if let Some(color) = day_background_color {
            decoration = decoration.color(color);
        }

        let day_number = localizations.format_decimal(i64::from(day.day()));
        let mut day_widget: WidgetRef = if let Some(day_builder) = day_builder {
            let default_day = Ink::new()
                .decoration(decoration)
                .child(Center::new().child(ExcludeSemantics::new().child(Text::new(day_number))))
                .into_widget();
            let listenable: Rc<dyn waft_foundation::Listenable> =
                Rc::new(states_controller.clone());
            let builder = ListenableBuilder::new(listenable, {
                let states_controller = states_controller.handle();
                move |app, context, child| {
                    let states = app.get(states_controller).value().clone();
                    day_builder(
                        app,
                        context,
                        CalendarDatePickerDayDetails {
                            day,
                            states,
                            child: child.expect("the default day").clone(),
                        },
                    )
                }
            })
            .child(default_day)
            .into_widget();
            let mut merge = DefaultTextStyle::merge(builder);
            if let Some(style) = apply_color(day_style, day_foreground_color) {
                merge = merge.style(style);
            }
            merge.into_widget()
        } else {
            let mut text = Text::new(day_number);
            if let Some(style) = apply_color(day_style, day_foreground_color) {
                text = text.style(style);
            }
            Ink::new()
                .decoration(decoration)
                .child(Center::new().child(ExcludeSemantics::new().child(text)))
                .into_widget()
        };

        // Adds padding as per M3 guidelines for portrait mode. Not applied in landscape
        // mode currently due to unclear specifications.
        let orientation = MediaQuery::orientation_of(app, context);
        if Theme::of(app, context).use_material3 && orientation == Orientation::Portrait {
            day_widget = Padding::new(EdgeInsetsGeometry::all(4.0))
                .child(day_widget)
                .into_widget();
        }
        day_widget = Semantics::new()
            // We want the day of month to be spoken first irrespective of the
            // locale-specific preferences or TextDirection. This is because
            // an accessibility user is more likely to be interested in the
            // day of month before the rest of the date, as they are looking
            // for the day of month. To do that we prepend day of month to the
            // formatted full date.
            .label(format!(
                "{}, {}{}",
                localizations.format_decimal(i64::from(day.day())),
                delegate.format_full_date(&day, &*localizations),
                semantic_label_suffix
            ))
            // Set button to true to make the date selectable.
            .button(true)
            .selected(is_selected_day)
            .enabled(!is_disabled)
            .child(day_widget)
            .into_widget();

        if !is_disabled {
            day_widget = InkResponse::new()
                .focus_node(focus_node.into_erased())
                .on_tap(Listener::new(move |app: &mut App| on_changed(app, day)))
                .states_controller(states_controller)
                .overlay_color(day_overlay_color)
                .custom_border(day_shape)
                .contained_ink_well(true)
                .child(day_widget)
                .into_widget();
        }

        day_widget
    }
}

impl fmt::Debug for DayState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DayState").finish_non_exhaustive()
    }
}

/// Dart's `_DayPickerGridDelegate`, with the text scale and the orientation its `context`
/// answers given when the widget builds it.
#[derive(Debug)]
struct DayPickerGridDelegate {
    text_scale_factor: f64,
    use_material3: bool,
    orientation: Orientation,
}

impl SliverGridDelegate for DayPickerGridDelegate {
    fn get_layout(&self, constraints: SliverConstraints) -> Box<dyn SliverGridLayout> {
        // Conform to M3 spec in portrait mode (landscape mode is not specified).
        let day_picker_row_height =
            if self.use_material3 && self.orientation == Orientation::Portrait {
                DAY_PICKER_ROW_HEIGHT_M3
            } else {
                DAY_PICKER_ROW_HEIGHT_M2
            };
        let scaled_row_height = if self.text_scale_factor > 1.3 {
            ((self.text_scale_factor - 1.0) * 30.0) + day_picker_row_height
        } else {
            day_picker_row_height
        };
        const COLUMN_COUNT: i32 = DateTime::DAYS_PER_WEEK;
        let tile_width = constraints.cross_axis_extent / f64::from(COLUMN_COUNT);
        let tile_height = scaled_row_height
            .min(constraints.viewport_main_axis_extent / f64::from(MAX_DAY_PICKER_ROW_COUNT + 1));
        Box::new(SliverGridRegularTileLayout::new(
            COLUMN_COUNT,
            tile_height,
            tile_width,
            tile_height,
            tile_width,
            axis_direction_is_reversed(constraints.cross_axis_direction),
        ))
    }

    fn should_relayout(&self, _old_delegate: &dyn SliverGridDelegate) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// A scrollable grid of years to allow picking a year.
///
/// The year picker widget is rarely used directly. Instead, consider using
/// [`CalendarDatePicker`], or `showDatePicker` which create full date pickers.
///
/// See also:
///
///  * `showDatePicker`, which shows a dialog that contains a Material Design date picker.
pub struct YearPicker {
    pub key: Option<KeyRef>,

    /// The current date.
    ///
    /// This date is subtly highlighted in the picker. Defaults to today; `None`: not given.
    pub current_date: Option<DateTime>,

    /// The earliest date the user is permitted to pick.
    pub first_date: DateTime,

    /// The latest date the user is permitted to pick.
    pub last_date: DateTime,

    /// The currently selected date.
    ///
    /// This date is highlighted in the picker.
    pub selected_date: Option<DateTime>,

    /// Called when the user picks a year.
    pub on_changed: ValueChanged<DateTime>,

    /// Determines the way that drag start behavior is handled.
    ///
    /// If set to [`DragStartBehavior::Start`], the drag behavior used for scrolling this widget
    /// will begin at the position where the drag gesture won the arena. If set to
    /// [`DragStartBehavior::Down`] it will begin at the position where a down event is first
    /// detected.
    ///
    /// Defaults to [`DragStartBehavior::Start`].
    pub drag_start_behavior: DragStartBehavior,

    /// The calendar system to use.
    ///
    /// Defaults to [`GregorianCalendarDelegate`].
    pub calendar_delegate: Rc<dyn CalendarDelegate>,
}

impl YearPicker {
    /// Creates a year picker; Dart's optional named arguments are the setters.
    ///
    /// The `first_date` is the earliest date the user may pick, and the `last_date` the
    /// latest. Dart's deprecated, ineffective `initialDate` has no counterpart.
    pub fn new(
        first_date: DateTime,
        last_date: DateTime,
        selected_date: Option<DateTime>,
        on_changed: ValueChanged<DateTime>,
    ) -> YearPicker {
        YearPicker {
            key: None,
            current_date: None,
            first_date,
            last_date,
            selected_date,
            on_changed,
            drag_start_behavior: DragStartBehavior::Start,
            calendar_delegate: Rc::new(GregorianCalendarDelegate),
        }
    }

    /// Dart's `currentDate` field, as its constructor fills it: the day of the date given, or of
    /// `DateTime.now()`.
    pub fn effective_current_date(&self) -> DateTime {
        let current_date = self.current_date.unwrap_or_else(DateTime::now);
        self.calendar_delegate.date_only(&current_date)
    }

    /// Dart `YearPicker(key:)`.
    pub fn key(mut self, key: KeyRef) -> YearPicker {
        self.key = Some(key);
        self
    }

    /// Dart `YearPicker(currentDate:)`.
    pub fn current_date(mut self, current_date: DateTime) -> YearPicker {
        self.current_date = Some(current_date);
        self
    }

    /// Dart `YearPicker(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> YearPicker {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `YearPicker(calendarDelegate:)`.
    pub fn calendar_delegate(mut self, calendar_delegate: Rc<dyn CalendarDelegate>) -> YearPicker {
        self.calendar_delegate = calendar_delegate;
        self
    }
}

impl fmt::Debug for YearPicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YearPicker")
            .field("current_date", &self.current_date)
            .field("first_date", &self.first_date)
            .field("last_date", &self.last_date)
            .field("selected_date", &self.selected_date)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for YearPicker {
    type State = YearPickerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(!self.first_date.is_after(&self.last_date));
    }

    fn create_state(&self) -> YearPickerState {
        YearPickerState {
            state: StateData::new(),
            scroll_controller: None,
            states_controller: None,
        }
    }
}

/// Dart's `_YearPickerState`.
pub struct YearPickerState {
    state: StateData<YearPicker>,
    /// Created in `init_state`, because a state is created without the [`App`].
    scroll_controller: Option<Ref<ScrollController>>,
    states_controller: Option<Ref<WidgetStatesController>>,
}

impl YearPickerState {
    // The approximate number of years necessary to fill the available space.
    const MIN_YEARS: i32 = 18;

    fn scroll_offset_for_year(self: Handle<Self>, app: &App, date: &DateTime) -> f64 {
        let widget = self.widget(app);
        let initial_year_index = date.year() - widget.first_date.year();
        let initial_year_row = initial_year_index.div_euclid(YEAR_PICKER_COLUMN_COUNT);
        // Move the offset down by 2 rows to approximately center it.
        let centered_year_row = initial_year_row - 2;
        if self.item_count(app) < Self::MIN_YEARS {
            0.0
        } else {
            f64::from(centered_year_row) * YEAR_PICKER_ROW_HEIGHT
        }
    }

    fn item_count(self: Handle<Self>, app: &App) -> i32 {
        let widget = self.widget(app);
        widget.last_date.year() - widget.first_date.year() + 1
    }

    fn build_year_item(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: i32,
    ) -> WidgetRef {
        let date_picker_theme = DatePickerTheme::of(app, context);
        let defaults = DatePickerTheme::defaults(app, context);
        let text_scale_factor = clamped_text_scale_factor(app, context, 3.0);

        let widget = self.widget(app);
        let delegate = widget.calendar_delegate.clone();
        let (first_date, last_date) = (widget.first_date, widget.last_date);
        let selected_date = widget.selected_date;
        let current_date = widget.effective_current_date();
        let on_changed = widget.on_changed.clone();

        // Backfill the _YearPicker with disabled years if necessary.
        let item_count = self.item_count(app);
        let offset = if item_count < Self::MIN_YEARS {
            (Self::MIN_YEARS - item_count) / 2
        } else {
            0
        };
        let year = first_date.year() + index - offset;
        let is_selected = Some(year) == selected_date.map(|date| date.year());
        let is_current_year = year == current_date.year();
        let is_disabled = year < first_date.year() || year > last_date.year();
        let decoration_height = 36.0 * text_scale_factor;
        let decoration_width = 72.0 * text_scale_factor;

        let mut states = WidgetStates::new();
        if is_disabled {
            states.insert(WidgetState::Disabled);
        }
        if is_selected {
            states.insert(WidgetState::Selected);
        }

        let text_color = if is_current_year {
            resolve_color(
                date_picker_theme.today_foreground_color.as_ref(),
                defaults.today_foreground_color(app),
                &states,
            )
        } else {
            resolve_color(
                date_picker_theme.year_foreground_color.as_ref(),
                defaults.year_foreground_color(app),
                &states,
            )
        };
        let background = if is_current_year {
            resolve_color(
                date_picker_theme.today_background_color.as_ref(),
                defaults.today_background_color(app),
                &states,
            )
        } else {
            resolve_color(
                date_picker_theme.year_background_color.as_ref(),
                defaults.year_background_color(app),
                &states,
            )
        };
        let theme_year_overlay_color = date_picker_theme.year_overlay_color.clone();
        let default_year_overlay_color = defaults.year_overlay_color(app);
        let overlay_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            resolve_color(
                theme_year_overlay_color.as_ref(),
                default_year_overlay_color.clone(),
                states,
            )
        });

        let year_shape = resolve_shape(
            date_picker_theme.year_shape.as_ref(),
            defaults.year_shape(app),
            &states,
        )
        .expect("the defaults have a year shape");

        let border_side = if is_current_year {
            today_border_side(
                date_picker_theme.today_border,
                defaults.today_border(app),
                text_color,
            )
        } else {
            None
        };
        let year_decoration_shape: ShapeBorderRef = match border_side {
            Some(side) => year_shape.copy_with(Some(side)).into(),
            None => year_shape.clone().into(),
        };
        let mut decoration = ShapeDecoration::new(year_decoration_shape);
        if let Some(color) = background {
            decoration = decoration.color(color);
        }

        let item_style = apply_color(
            date_picker_theme
                .year_style
                .clone()
                .or_else(|| defaults.year_style(app)),
            text_color,
        );
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let mut year_text = Text::new(delegate.format_year(year, &*localizations));
        if let Some(style) = item_style {
            year_text = year_text.style(style);
        }
        let mut year_item = Center::new()
            .child(
                waft_widgets::Container::new()
                    .decoration(decoration)
                    .height(decoration_height)
                    .width(decoration_width)
                    .alignment(waft_painting::Alignment::CENTER.into())
                    .child(
                        Semantics::new()
                            .selected(is_selected)
                            .enabled(!is_disabled)
                            .button(true)
                            .child(year_text),
                    ),
            )
            .into_widget();

        if !is_disabled {
            let mut date = delegate.get_month(
                year,
                selected_date.map_or(DateTime::JANUARY, |date| date.month()),
            );
            if date.is_before(&delegate.get_month(first_date.year(), first_date.month())) {
                // Ignore firstDate.day because we're just working in years and months here.
                debug_assert_eq!(date.year(), first_date.year());
                date = delegate.get_month(year, first_date.month());
            } else if date.is_after(&last_date) {
                // No need to ignore the day here because it can only be bigger than what we
                // care about.
                debug_assert_eq!(date.year(), last_date.year());
                date = delegate.get_month(year, last_date.month());
            }
            let states_controller = self.states_controller_ref(app);
            states_controller.handle().set_value(app, states);
            year_item = InkWell::new()
                .key(waft_foundation::ValueKey::new(year).into())
                .on_tap(Listener::new(move |app: &mut App| on_changed(app, date)))
                .states_controller(states_controller)
                .overlay_color(overlay_color)
                .child(year_item)
                .into_widget();
        }

        year_item
    }

    fn states_controller_ref(self: Handle<Self>, app: &App) -> Ref<WidgetStatesController> {
        app.get(self)
            .states_controller
            .clone()
            .expect("created in init_state")
    }

    fn scroll_controller(self: Handle<Self>, app: &App) -> Ref<ScrollController> {
        app.get(self)
            .scroll_controller
            .clone()
            .expect("created in init_state")
    }
}

impl State for YearPickerState {
    type Widget = YearPicker;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let date = widget.selected_date.unwrap_or(widget.first_date);
        let initial_scroll_offset = self.scroll_offset_for_year(app, &date);
        let scroll_controller =
            ScrollController::new(app, initial_scroll_offset, true, None, None, None);
        let states_controller = WidgetStatesController::new(app, None);
        let state = app.get_mut(self);
        state.scroll_controller = Some(scroll_controller);
        state.states_controller = Some(states_controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.scroll_controller(app).handle().dispose(app);
        self.states_controller_ref(app).handle().dispose(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &YearPicker) {
        let selected_date = self.widget(app).selected_date;
        if selected_date != old_widget.selected_date
            && let Some(selected_date) = selected_date
        {
            let offset = self.scroll_offset_for_year(app, &selected_date);
            self.scroll_controller(app).handle().jump_to(app, offset);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_scale_factor = clamped_text_scale_factor(app, context, 3.0);
        let drag_start_behavior = self.widget(app).drag_start_behavior;
        let item_count = self.item_count(app).max(Self::MIN_YEARS);
        let scroll_controller = self.scroll_controller(app);
        Column::new()
            .children([
                Divider::new().into_widget(),
                Expanded::new(
                    Material::new().r#type(MaterialType::Transparency).child(
                        GridView::builder(
                            Rc::new(YearPickerGridDelegate { text_scale_factor }),
                            move |app, context, index| {
                                Some(self.build_year_item(app, context, index))
                            },
                        )
                        .item_count(item_count)
                        .build()
                        .controller(scroll_controller.erase())
                        .drag_start_behavior(drag_start_behavior)
                        .padding(EdgeInsetsGeometry::symmetric(0.0, YEAR_PICKER_PADDING)),
                    ),
                )
                .into_widget(),
                Divider::new().into_widget(),
            ])
            .into_widget()
    }
}

impl fmt::Debug for YearPickerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YearPickerState").finish_non_exhaustive()
    }
}

/// Dart's `_YearPickerGridDelegate`, with the text scale its `context` answers given when the
/// widget builds it.
#[derive(Debug)]
struct YearPickerGridDelegate {
    text_scale_factor: f64,
}

impl SliverGridDelegate for YearPickerGridDelegate {
    fn get_layout(&self, constraints: SliverConstraints) -> Box<dyn SliverGridLayout> {
        let scaled_year_picker_column_count = if self.text_scale_factor > 1.65 {
            YEAR_PICKER_COLUMN_COUNT - 1
        } else {
            YEAR_PICKER_COLUMN_COUNT
        };
        let tile_width = ((constraints.cross_axis_extent
            - f64::from(scaled_year_picker_column_count - 1) * YEAR_PICKER_ROW_SPACING)
            / f64::from(scaled_year_picker_column_count))
        .max(0.0);
        let scaled_year_picker_row_height = if self.text_scale_factor > 1.0 {
            YEAR_PICKER_ROW_HEIGHT + ((self.text_scale_factor - 1.0) * 9.0)
        } else {
            YEAR_PICKER_ROW_HEIGHT
        };
        Box::new(SliverGridRegularTileLayout::new(
            scaled_year_picker_column_count,
            scaled_year_picker_row_height,
            tile_width + YEAR_PICKER_ROW_SPACING,
            scaled_year_picker_row_height,
            tile_width,
            axis_direction_is_reversed(constraints.cross_axis_direction),
        ))
    }

    fn should_relayout(&self, _old_delegate: &dyn SliverGridDelegate) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};

    use waft_painting::TextDirection;
    use waft_services::PhysicalKeyboardKey;
    use waft_widgets::{AnyElement, Directionality, IntoWidget};

    use super::*;
    use crate::chip::tests::{center_of, tester};
    use crate::test_support::{
        PaintInspection, WidgetTester, material_app, with_main_thread_stack,
    };
    use crate::theme_data::ThemeData;

    /// The options of Dart's test helper `calendarDatePicker(..)`.
    #[derive(Clone)]
    pub(crate) struct PickerOptions {
        pub(crate) initial_date: Option<DateTime>,
        pub(crate) first_date: DateTime,
        pub(crate) last_date: DateTime,
        pub(crate) current_date: DateTime,
        pub(crate) on_date_changed: ValueChanged<DateTime>,
        pub(crate) on_displayed_month_changed: Option<ValueChanged<DateTime>>,
        pub(crate) initial_calendar_mode: DatePickerMode,
        pub(crate) selectable_day_predicate: Option<SelectableDayPredicate>,
        pub(crate) day_builder: Option<CalendarDatePickerDayBuilder>,
        pub(crate) weekday_builder: Option<CalendarDatePickerWeekdayBuilder>,
        pub(crate) text_direction: TextDirection,
        pub(crate) use_material3: Option<bool>,
        pub(crate) theme: Option<ThemeData>,
        pub(crate) key: Option<GlobalKey>,
    }

    impl Default for PickerOptions {
        fn default() -> PickerOptions {
            PickerOptions {
                initial_date: None,
                first_date: DateTime::utc(2001, 1, 1),
                last_date: DateTime::utc(2031, 12, 31),
                current_date: DateTime::utc(2016, 1, 3),
                on_date_changed: Rc::new(|_app, _date| {}),
                on_displayed_month_changed: None,
                initial_calendar_mode: DatePickerMode::Day,
                selectable_day_predicate: None,
                day_builder: None,
                weekday_builder: None,
                text_direction: TextDirection::Ltr,
                use_material3: None,
                theme: None,
                key: None,
            }
        }
    }

    /// What a picker reported: Dart's `DateTime? selectedDate` and `displayedMonth`.
    #[derive(Clone, Default)]
    pub(crate) struct Reported {
        pub(crate) selected_date: Rc<Cell<Option<DateTime>>>,
        pub(crate) displayed_month: Rc<Cell<Option<DateTime>>>,
    }

    impl Reported {
        pub(crate) fn options(&self) -> PickerOptions {
            let selected_date = Rc::clone(&self.selected_date);
            let displayed_month = Rc::clone(&self.displayed_month);
            PickerOptions {
                on_date_changed: Rc::new(move |_app, date| selected_date.set(Some(date))),
                on_displayed_month_changed: Some(Rc::new(move |_app, date| {
                    displayed_month.set(Some(date))
                })),
                ..PickerOptions::default()
            }
        }
    }

    pub(crate) fn theme_for(tester: &WidgetTester, use_material3: Option<bool>) -> ThemeData {
        let app = tester.cell.borrow();
        match use_material3 {
            Some(false) => ThemeData::builder().use_material3(false).build(&app),
            _ => crate::tooltip::tests::m3(&app),
        }
    }

    /// Dart's `calendarDatePicker(..)`.
    pub(crate) fn calendar_date_picker(tester: &WidgetTester, options: PickerOptions) -> WidgetRef {
        let mut picker = CalendarDatePicker::new(
            options.initial_date,
            options.first_date,
            options.last_date,
            options.on_date_changed.clone(),
        )
        .current_date(options.current_date)
        .initial_calendar_mode(options.initial_calendar_mode);
        if let Some(key) = options.key.clone() {
            picker = picker.key(key.into());
        }
        if let Some(on_displayed_month_changed) = options.on_displayed_month_changed.clone() {
            picker = picker.on_displayed_month_changed(on_displayed_month_changed);
        }
        if let Some(predicate) = options.selectable_day_predicate.clone() {
            picker = picker.selectable_day_predicate(predicate);
        }
        if let Some(day_builder) = options.day_builder.clone() {
            picker = picker.day_builder(day_builder);
        }
        if let Some(weekday_builder) = options.weekday_builder.clone() {
            picker = picker.weekday_builder(weekday_builder);
        }
        material_app(
            options
                .theme
                .clone()
                .unwrap_or_else(|| theme_for(tester, options.use_material3)),
            Material::new()
                .child(Directionality::new(options.text_direction, picker))
                .into_widget(),
        )
    }

    /// Dart's `yearPicker(..)`.
    pub(crate) fn year_picker(
        tester: &WidgetTester,
        selected_date: Option<DateTime>,
        first_date: Option<DateTime>,
        last_date: Option<DateTime>,
        on_changed: Option<ValueChanged<DateTime>>,
    ) -> WidgetRef {
        let picker = YearPicker::new(
            first_date.unwrap_or(DateTime::utc(2001, 1, 1)),
            last_date.unwrap_or(DateTime::utc(2031, 12, 31)),
            Some(selected_date.unwrap_or(DateTime::utc(2016, 1, 15))),
            on_changed.unwrap_or_else(|| Rc::new(|_app, _date| {})),
        )
        .current_date(DateTime::utc(2016, 1, 3));
        material_app(
            theme_for(tester, None),
            Material::new()
                .child(Directionality::new(TextDirection::Ltr, picker))
                .into_widget(),
        )
    }

    /// The first element showing `text`, Dart's `find.text(text)` for a tap.
    pub(crate) fn text(tester: &WidgetTester, text: &str) -> AnyElement {
        *tester
            .find_text(text)
            .first()
            .unwrap_or_else(|| panic!("a Text showing {text:?}"))
    }

    pub(crate) fn has_text(tester: &WidgetTester, text: &str) -> bool {
        !tester.find_text(text).is_empty()
    }

    pub(crate) fn tap_text(tester: &mut WidgetTester, label: &str) {
        let element = text(tester, label);
        let at = center_of(tester, element);
        tester.tap(at, 1);
    }

    /// Dart's `nextMonthIcon` and `previousMonthIcon`: the `IconButton` whose tooltip, or whose
    /// icon's semantic label, starts with `prefix`.
    pub(crate) fn month_button(tester: &WidgetTester, prefix: &str) -> AnyElement {
        tester
            .find_by_type::<IconButton>()
            .into_iter()
            .find(|button| {
                let tooltip = tester.widget(*button, |button: &IconButton| button.tooltip.clone());
                if tooltip.is_some_and(|tooltip| tooltip.starts_with(prefix)) {
                    return true;
                }
                tester
                    .descendants_of_type::<Icon>(*button)
                    .into_iter()
                    .any(|icon| {
                        tester
                            .widget(icon, |icon: &Icon| icon.semantic_label.clone())
                            .is_some_and(|label| label.starts_with(prefix))
                    })
            })
            .unwrap_or_else(|| panic!("an IconButton labeled {prefix:?}"))
    }

    pub(crate) fn tap_element(tester: &mut WidgetTester, element: AnyElement) {
        let at = center_of(tester, element);
        tester.tap(at, 1);
    }

    pub(crate) fn tap_month_button(tester: &mut WidgetTester, prefix: &str) {
        let button = month_button(tester, prefix);
        tap_element(tester, button);
    }

    /// Whether the month button is enabled: Dart's `getSemantics(..).flagsCollection.isEnabled`.
    pub(crate) fn month_button_enabled(tester: &WidgetTester, prefix: &str) -> bool {
        let button = month_button(tester, prefix);
        tester.widget(button, |button: &IconButton| button.on_pressed.is_some())
    }

    fn pump(tester: &mut WidgetTester, options: PickerOptions) {
        let root = calendar_date_picker(tester, options);
        tester.pump_widget(root);
    }

    #[test]
    fn can_select_a_day() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..reported.options()
                },
            );
            tap_text(&mut tester, "12");
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2016, 1, 12))
            );
        });
    }

    #[test]
    fn can_select_a_day_with_nothing_first_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(&mut tester, reported.options());
            tap_text(&mut tester, "12");
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2016, 1, 12))
            );
        });
    }

    fn run_can_select_a_month(initial_date: Option<DateTime>) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date,
                    ..reported.options()
                },
            );
            assert!(has_text(&tester, "January 2016"));

            // Go back two months
            tap_month_button(&mut tester, "Previous month");
            tester.pump_and_settle();
            assert!(has_text(&tester, "December 2015"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2015, 12, 1))
            );
            tap_month_button(&mut tester, "Previous month");
            tester.pump_and_settle();
            assert!(has_text(&tester, "November 2015"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2015, 11, 1))
            );

            // Go forward a month
            tap_month_button(&mut tester, "Next month");
            tester.pump_and_settle();
            assert!(has_text(&tester, "December 2015"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2015, 12, 1))
            );
        });
    }

    #[test]
    fn can_select_a_month() {
        run_can_select_a_month(Some(DateTime::utc(2016, 1, 15)));
    }

    #[test]
    fn can_select_a_month_with_nothing_first_selected() {
        run_can_select_a_month(None);
    }

    fn run_can_select_a_year(initial_date: Option<DateTime>) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date,
                    ..reported.options()
                },
            );

            tap_text(&mut tester, "January 2016"); // Switch to year mode.
            tester.pump_and_settle();
            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            assert!(has_text(&tester, "January 2018"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2018, 1, 1))
            );
        });
    }

    #[test]
    fn can_select_a_year() {
        run_can_select_a_year(Some(DateTime::utc(2016, 1, 15)));
    }

    #[test]
    fn can_select_a_year_with_nothing_first_selected() {
        run_can_select_a_year(None);
    }

    #[test]
    fn selecting_date_does_not_change_displayed_month() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2020, 3, 15)),
                    ..reported.options()
                },
            );

            tap_month_button(&mut tester, "Next month");
            tester.pump_and_settle();
            assert!(has_text(&tester, "April 2020"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2020, 4, 1))
            );

            tap_text(&mut tester, "25");
            tester.pump_and_settle();
            assert!(has_text(&tester, "April 2020"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2020, 4, 1))
            );
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2020, 4, 25))
            );
            // There isn't a 31 in April so there shouldn't be one if it is showing April.
            assert!(!has_text(&tester, "31"));
        });
    }

    #[test]
    fn changing_year_does_change_selected_date() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..reported.options()
                },
            );
            tap_text(&mut tester, "4");
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2016, 1, 4))
            );
            tap_text(&mut tester, "January 2016");
            tester.pump_and_settle();
            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2018, 1, 4))
            );
        });
    }

    #[test]
    fn changing_year_for_february_29th() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2020, 2, 29)),
                    ..reported.options()
                },
            );
            tap_text(&mut tester, "February 2020");
            tester.pump_and_settle();
            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2018, 2, 28))
            );
            tap_text(&mut tester, "February 2018");
            tester.pump_and_settle();
            tap_text(&mut tester, "2020");
            tester.pump_and_settle();
            // Changing back to 2020 the 29th is not selected anymore.
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2020, 2, 28))
            );
        });
    }

    #[test]
    fn changing_year_does_not_change_the_month() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..reported.options()
                },
            );
            tap_month_button(&mut tester, "Next month");
            tester.pump_and_settle();
            tap_month_button(&mut tester, "Next month");
            tester.pump_and_settle();
            tap_text(&mut tester, "March 2016");
            tester.pump_and_settle();
            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            assert!(has_text(&tester, "March 2018"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2018, 3, 1))
            );
        });
    }

    #[test]
    fn can_select_a_year_and_then_a_day() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..reported.options()
                },
            );
            tap_text(&mut tester, "January 2016"); // Switch to year mode.
            tester.pump_and_settle();
            tap_text(&mut tester, "2017");
            tester.pump_and_settle();
            tap_text(&mut tester, "19");
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2017, 1, 19))
            );
        });
    }

    #[test]
    fn cannot_select_a_day_outside_bounds() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let valid_date = DateTime::utc(2017, 1, 15);
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(valid_date),
                    first_date: valid_date,
                    last_date: valid_date,
                    ..reported.options()
                },
            );

            // Earlier than firstDate. Should be ignored.
            tap_text(&mut tester, "10");
            assert_eq!(reported.selected_date.get(), None);

            // Later than lastDate. Should be ignored.
            tap_text(&mut tester, "20");
            assert_eq!(reported.selected_date.get(), None);

            // This one is just right.
            tap_text(&mut tester, "15");
            assert_eq!(reported.selected_date.get(), Some(valid_date));
        });
    }

    #[test]
    fn cannot_navigate_to_a_month_outside_bounds() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2016, 12, 15),
                    initial_date: Some(DateTime::utc(2017, 1, 15)),
                    last_date: DateTime::utc(2017, 2, 15),
                    ..reported.options()
                },
            );

            tap_month_button(&mut tester, "Next month");
            tester.pump_and_settle();
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2017, 2, 1))
            );
            // Shouldn't be possible to keep going forward into March.
            assert!(!month_button_enabled(&tester, "Next month"));

            tap_month_button(&mut tester, "Previous month");
            tester.pump_and_settle();
            tap_month_button(&mut tester, "Previous month");
            tester.pump_and_settle();
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2016, 12, 1))
            );
            // Shouldn't be possible to keep going backward into November.
            assert!(!month_button_enabled(&tester, "Previous month"));
        });
    }

    #[test]
    fn cannot_select_disabled_year() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2018, 6, 9),
                    initial_date: Some(DateTime::utc(2018, 7, 4)),
                    last_date: DateTime::utc(2018, 12, 15),
                    ..reported.options()
                },
            );
            tap_text(&mut tester, "July 2018"); // Switch to year mode.
            tester.pump_and_settle();
            tap_text(&mut tester, "2016"); // Disabled, doesn't change the year.
            tester.pump_and_settle();
            tap_text(&mut tester, "2020"); // Disabled, doesn't change the year.
            tester.pump_and_settle();

            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            // Nothing should have changed.
            assert_eq!(reported.displayed_month.get(), None);
        });
    }

    #[test]
    fn selecting_first_date_year_respects_first_date() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2016, 6, 9),
                    initial_date: Some(DateTime::utc(2018, 5, 4)),
                    last_date: DateTime::utc(2019, 1, 15),
                    ..reported.options()
                },
            );
            tap_text(&mut tester, "May 2018");
            tester.pump_and_settle();
            tap_text(&mut tester, "2016");
            tester.pump_and_settle();
            // Month should be clamped to June as the range starts at June 2016.
            assert!(has_text(&tester, "June 2016"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2016, 6, 1))
            );
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2016, 6, 9))
            );
        });
    }

    #[test]
    fn selecting_last_date_year_respects_last_date() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2016, 6, 9),
                    initial_date: Some(DateTime::utc(2018, 5, 4)),
                    last_date: DateTime::utc(2019, 1, 15),
                    ..reported.options()
                },
            );
            // Selected date is now 2018-05-04 (initialDate).
            tap_text(&mut tester, "May 2018");
            // Selected date is still 2018-05-04.
            tester.pump_and_settle();
            tap_text(&mut tester, "2019");
            // Selected date would become 2019-05-04 but gets clamped to the month of
            // lastDate, so 2019-01-04.
            tester.pump_and_settle();
            assert!(has_text(&tester, "January 2019"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2019, 1, 1))
            );
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2019, 1, 4))
            );
        });
    }

    #[test]
    fn selecting_last_date_year_respects_last_date_when_the_day_is_beyond_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2016, 6, 9),
                    initial_date: Some(DateTime::utc(2018, 5, 15)),
                    last_date: DateTime::utc(2019, 1, 4),
                    ..reported.options()
                },
            );
            // Selected date is now 2018-05-15 (initialDate).
            tap_text(&mut tester, "May 2018");
            // Selected date is still 2018-05-15.
            tester.pump_and_settle();
            tap_text(&mut tester, "2019");
            // Selected date would become 2019-05-15 but gets clamped to the month of
            // lastDate, so 2019-01-15. Day is now beyond the lastDate so that also gets
            // clamped, to 2019-01-04.
            tester.pump_and_settle();
            assert!(has_text(&tester, "January 2019"));
            assert_eq!(
                reported.displayed_month.get(),
                Some(DateTime::utc(2019, 1, 1))
            );
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2019, 1, 4))
            );
        });
    }

    #[test]
    fn only_predicate_days_are_selectable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2017, 1, 10),
                    initial_date: Some(DateTime::utc(2017, 1, 16)),
                    last_date: DateTime::utc(2017, 1, 20),
                    selectable_day_predicate: Some(Rc::new(|date: DateTime| date.day() % 2 == 0)),
                    ..reported.options()
                },
            );
            tap_text(&mut tester, "13"); // Odd, doesn't work.
            assert_eq!(reported.selected_date.get(), None);
            tap_text(&mut tester, "10"); // Even, works.
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2017, 1, 10))
            );
            tap_text(&mut tester, "17"); // Odd, doesn't work.
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2017, 1, 10))
            );
        });
    }

    #[test]
    fn can_select_initial_calendar_picker_mode() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2014, 1, 15)),
                    initial_calendar_mode: DatePickerMode::Year,
                    ..PickerOptions::default()
                },
            );
            // 2018 wouldn't be available if the year picker wasn't showing.
            // The initial current year is 2014.
            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            assert!(has_text(&tester, "January 2018"));
        });
    }

    fn custom_day_builder(
        states_for_day: Rc<RefCell<HashMap<DateTime, WidgetStates>>>,
    ) -> CalendarDatePickerDayBuilder {
        Rc::new(move |_app, _context, details| {
            states_for_day
                .borrow_mut()
                .insert(details.day, details.states.clone());
            details.child
        })
    }

    #[test]
    fn day_builder_customizes_days_and_preserves_tap_handling() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected_dates = Rc::new(RefCell::new(Vec::new()));
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    on_date_changed: {
                        let selected_dates = Rc::clone(&selected_dates);
                        Rc::new(move |_app, date| selected_dates.borrow_mut().push(date))
                    },
                    day_builder: Some(Rc::new(|_app, _context, details| match details.day.day() {
                        1 => Text::new("Custom day 1").into_widget(),
                        12 => Semantics::new()
                            .label("Custom semantics")
                            .child(details.child)
                            .into_widget(),
                        _ => details.child,
                    })),
                    ..PickerOptions::default()
                },
            );

            assert_eq!(tester.find_text("Custom day 1").len(), 1);

            // Notify a date change only when the custom day is tapped.
            assert!(selected_dates.borrow().is_empty());
            tap_text(&mut tester, "Custom day 1");
            tester.pump_and_settle();
            assert_eq!(*selected_dates.borrow(), vec![DateTime::utc(2016, 1, 1)]);
        });
    }

    #[test]
    fn day_builder_receives_day_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let states_for_day = Rc::new(RefCell::new(HashMap::new()));
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    selectable_day_predicate: Some(Rc::new(|date: DateTime| date.day() != 10)),
                    day_builder: Some(custom_day_builder(Rc::clone(&states_for_day))),
                    ..PickerOptions::default()
                },
            );

            let states = |day: i32| {
                states_for_day
                    .borrow()
                    .get(&DateTime::utc(2016, 1, day))
                    .cloned()
                    .unwrap_or_default()
            };
            assert!(states(10).contains(&WidgetState::Disabled));
            assert!(states(15).contains(&WidgetState::Selected));

            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::new(0.0, 0.0));
            let twelve = text(&tester, "12");
            let at = center_of(&tester, twelve);
            tester.move_to(&mut mouse, at, false);
            tester.pump(Duration::ZERO);
            assert!(states(12).contains(&WidgetState::Hovered));

            tap_text(&mut tester, "12");
            tester.pump_and_settle();
            assert!(states(12).contains(&WidgetState::Selected));
            assert!(!states(15).contains(&WidgetState::Selected));
        });
    }

    #[test]
    fn weekday_builder_customizes_headers_and_uses_date_time_weekday_values() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    weekday_builder: Some(Rc::new(|_app, _context, details| {
                        if details.weekday == DateTime::SUNDAY {
                            Text::new("Custom Sunday").into_widget()
                        } else {
                            details.child
                        }
                    })),
                    ..PickerOptions::default()
                },
            );
            assert_eq!(tester.find_text("Custom Sunday").len(), 1);
        });
    }

    #[test]
    fn weekday_builder_receives_weekdays_in_locale_order_for_en_us() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let weekdays = Rc::new(RefCell::new(Vec::new()));
            pump(
                &mut tester,
                PickerOptions {
                    weekday_builder: Some({
                        let weekdays = Rc::clone(&weekdays);
                        Rc::new(move |_app, _context, details| {
                            weekdays.borrow_mut().push(details.weekday);
                            details.child
                        })
                    }),
                    ..PickerOptions::default()
                },
            );
            assert_eq!(
                *weekdays.borrow(),
                vec![
                    DateTime::SUNDAY,
                    DateTime::MONDAY,
                    DateTime::TUESDAY,
                    DateTime::WEDNESDAY,
                    DateTime::THURSDAY,
                    DateTime::FRIDAY,
                    DateTime::SATURDAY,
                ]
            );
        });
    }

    #[test]
    fn selecting_date_does_not_switch_picker_to_year_selection() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2020, 5, 10)),
                    initial_calendar_mode: DatePickerMode::Year,
                    ..PickerOptions::default()
                },
            );
            tap_text(&mut tester, "2017");
            tester.pump_and_settle();
            assert!(has_text(&tester, "May 2017"));
            tap_text(&mut tester, "10");
            tester.pump_and_settle();
            assert!(has_text(&tester, "May 2017"));
            assert!(!has_text(&tester, "2017"));
        });
    }

    #[test]
    fn selecting_disabled_date_does_not_change_current_selection() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let day = |day: i32| DateTime::utc(2020, 5, day);
            let selection = Rc::new(Cell::new(day(2)));
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(selection.get()),
                    first_date: day(2),
                    last_date: day(3),
                    on_date_changed: {
                        let selection = Rc::clone(&selection);
                        Rc::new(move |_app, date| selection.set(date))
                    },
                    ..PickerOptions::default()
                },
            );

            tap_text(&mut tester, "3");
            tester.pump_and_settle();
            assert_eq!(selection.get(), day(3));
            tap_text(&mut tester, "4");
            tester.pump_and_settle();
            assert_eq!(selection.get(), day(3));
            tap_text(&mut tester, "5");
            tester.pump_and_settle();
            assert_eq!(selection.get(), day(3));
        });
    }

    #[test]
    fn updates_to_initial_calendar_mode_parameter_is_not_reflected_in_the_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let picker_key = GlobalKey::new();

            pump(
                &mut tester,
                PickerOptions {
                    key: Some(picker_key.clone()),
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    initial_calendar_mode: DatePickerMode::Year,
                    ..PickerOptions::default()
                },
            );
            tester.pump_and_settle();

            // Should be in year mode.
            assert_eq!(tester.find_text("January 2016").len(), 1); // Day/year selector
            assert!(!has_text(&tester, "15")); // day 15 in grid
            assert_eq!(tester.find_text("2016").len(), 1); // 2016 in year grid

            pump(
                &mut tester,
                PickerOptions {
                    key: Some(picker_key),
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..PickerOptions::default()
                },
            );
            tester.pump_and_settle();

            // Should be in year mode still; updating an _initial_ parameter has no effect.
            assert_eq!(tester.find_text("January 2016").len(), 1); // Day/year selector
            assert!(!has_text(&tester, "15")); // day 15 in grid
            assert_eq!(tester.find_text("2016").len(), 1); // 2016 in year grid
        });
    }

    fn press(
        tester: &mut WidgetTester,
        physical: PhysicalKeyboardKey,
        logical: LogicalKeyboardKey,
    ) {
        tester.send_key_event(physical, logical);
    }

    fn tab(tester: &mut WidgetTester) {
        press(tester, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
    }

    fn space(tester: &mut WidgetTester) {
        press(
            tester,
            PhysicalKeyboardKey::SPACE,
            LogicalKeyboardKey::SPACE,
        );
    }

    fn arrow(tester: &mut WidgetTester, direction: TraversalDirection) {
        let (physical, logical) = match direction {
            TraversalDirection::Left => (
                PhysicalKeyboardKey::ARROW_LEFT,
                LogicalKeyboardKey::ARROW_LEFT,
            ),
            TraversalDirection::Right => (
                PhysicalKeyboardKey::ARROW_RIGHT,
                LogicalKeyboardKey::ARROW_RIGHT,
            ),
            TraversalDirection::Up => (PhysicalKeyboardKey::ARROW_UP, LogicalKeyboardKey::ARROW_UP),
            TraversalDirection::Down => (
                PhysicalKeyboardKey::ARROW_DOWN,
                LogicalKeyboardKey::ARROW_DOWN,
            ),
        };
        press(tester, physical, logical);
    }

    #[test]
    fn keyboard_can_toggle_to_year_mode() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..PickerOptions::default()
                },
            );
            assert!(!has_text(&tester, "2016"));
            assert_eq!(tester.find_text("January 2016").len(), 1);
            // Navigate to the year selector and activate it.
            tab(&mut tester);
            space(&mut tester);
            tester.pump_and_settle();
            // The years should be visible.
            assert_eq!(tester.find_text("2016").len(), 1);
            assert_eq!(tester.find_text("January 2016").len(), 1);
        });
    }

    #[test]
    fn keyboard_can_navigate_next_and_previous_months() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..PickerOptions::default()
                },
            );
            assert_eq!(tester.find_text("January 2016").len(), 1);
            // Navigate to the previous month button and activate it twice.
            tab(&mut tester);
            tab(&mut tester);
            space(&mut tester);
            tester.pump_and_settle();
            space(&mut tester);
            tester.pump_and_settle();
            // Should be showing Nov 2015
            assert_eq!(tester.find_text("November 2015").len(), 1);

            // Navigate to the next month button and activate it four times.
            tab(&mut tester);
            for _ in 0..4 {
                space(&mut tester);
                tester.pump_and_settle();
            }
            // Should be on Mar 2016.
            assert_eq!(tester.find_text("March 2016").len(), 1);
        });
    }

    fn navigate_to_the_grid(tester: &mut WidgetTester) {
        for _ in 0..4 {
            tab(tester);
        }
    }

    #[test]
    fn keyboard_can_navigate_date_grid_with_arrow_keys() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..reported.options()
                },
            );
            navigate_to_the_grid(&mut tester);

            // Navigate from Jan 15 to Jan 18 with arrow keys.
            for direction in [
                TraversalDirection::Left,
                TraversalDirection::Up,
                TraversalDirection::Left,
                TraversalDirection::Down,
                TraversalDirection::Down,
                TraversalDirection::Left,
                TraversalDirection::Left,
            ] {
                arrow(&mut tester, direction);
            }
            tester.pump_and_settle();

            // Activate it.
            space(&mut tester);
            tester.pump_and_settle();

            // Should have selected Jan 18.
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2016, 1, 18))
            );
        });
    }

    #[test]
    fn keyboard_navigating_with_arrow_keys_scrolls_months() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..reported.options()
                },
            );
            navigate_to_the_grid(&mut tester);
            tester.pump_and_settle();

            // Navigate from Jan 15 to Dec 31 with arrow keys
            arrow(&mut tester, TraversalDirection::Up);
            arrow(&mut tester, TraversalDirection::Up);
            arrow(&mut tester, TraversalDirection::Left);
            tester.pump_and_settle();

            // Should have scrolled to Dec 2015.
            assert_eq!(tester.find_text("December 2015").len(), 1);

            // Navigate from Dec 31 to Nov 26 with arrow keys.
            for _ in 0..5 {
                arrow(&mut tester, TraversalDirection::Up);
            }
            tester.pump_and_settle();

            // Should have scrolled to Nov 2015.
            assert_eq!(tester.find_text("November 2015").len(), 1);

            // Activate it
            space(&mut tester);
            tester.pump_and_settle();

            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2015, 11, 26))
            );
        });
    }

    #[test]
    fn keyboard_rtl_text_direction_reverses_the_horizontal_arrow_key_navigation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let reported = Reported::default();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    text_direction: TextDirection::Rtl,
                    ..reported.options()
                },
            );
            navigate_to_the_grid(&mut tester);
            tester.pump_and_settle();

            // Navigate from Jan 15 to 19 with arrow keys.
            for _ in 0..4 {
                arrow(&mut tester, TraversalDirection::Right);
            }
            arrow(&mut tester, TraversalDirection::Down);
            arrow(&mut tester, TraversalDirection::Left);
            tester.pump_and_settle();

            // Activate it.
            space(&mut tester);
            tester.pump_and_settle();

            // Should have selected Jan 19.
            assert_eq!(
                reported.selected_date.get(),
                Some(DateTime::utc(2016, 1, 19))
            );
        });
    }

    #[test]
    fn calendar_date_picker_renders_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for mode in [DatePickerMode::Day, DatePickerMode::Year] {
                let picker = CalendarDatePicker::new(
                    Some(DateTime::utc(2025, 1, 1)),
                    DateTime::utc(2024, 1, 1),
                    DateTime::utc(2026, 1, 1),
                    Rc::new(|_app, _date| {}),
                )
                .initial_calendar_mode(mode);
                let theme = theme_for(&tester, None);
                tester.pump_widget(material_app(
                    theme,
                    crate::scaffold::Scaffold::new()
                        .body(SizedBox::shrink().child(picker))
                        .into_widget(),
                ));
            }
        });
    }

    #[test]
    fn year_picker_current_year_is_visible() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = year_picker(&tester, None, None, None, None);
            tester.pump_widget(root);
            assert_eq!(tester.find_text("2016").len(), 1);
        });
    }

    #[test]
    fn year_picker_can_select_a_year() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected_date = Rc::new(Cell::new(None));
            let root = year_picker(
                &tester,
                None,
                None,
                None,
                Some({
                    let selected_date = Rc::clone(&selected_date);
                    Rc::new(move |_app, date| selected_date.set(Some(date)))
                }),
            );
            tester.pump_widget(root);
            tester.pump_and_settle();
            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            assert_eq!(selected_date.get(), Some(DateTime::utc(2018, 1, 1)));
        });
    }

    #[test]
    fn year_picker_cannot_select_disabled_year() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected_year = Rc::new(Cell::new(None));
            let root = year_picker(
                &tester,
                Some(DateTime::utc(2018, 7, 4)),
                Some(DateTime::utc(2018, 6, 9)),
                Some(DateTime::utc(2018, 12, 15)),
                Some({
                    let selected_year = Rc::clone(&selected_year);
                    Rc::new(move |_app, date| selected_year.set(Some(date)))
                }),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, "2016"); // Disabled, doesn't change the year.
            tester.pump_and_settle();
            assert_eq!(selected_year.get(), None);
            tap_text(&mut tester, "2020"); // Disabled, doesn't change the year.
            tester.pump_and_settle();
            assert_eq!(selected_year.get(), None);
            tap_text(&mut tester, "2018");
            tester.pump_and_settle();
            assert_eq!(selected_year.get(), Some(DateTime::utc(2018, 7, 1)));
        });
    }

    #[test]
    fn year_picker_selecting_year_with_no_selected_month_uses_earliest_month() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected_year = Rc::new(Cell::new(None));
            let on_changed: ValueChanged<DateTime> = {
                let selected_year = Rc::clone(&selected_year);
                Rc::new(move |_app, date| selected_year.set(Some(date)))
            };
            let root = year_picker(
                &tester,
                None,
                Some(DateTime::utc(2018, 6, 9)),
                Some(DateTime::utc(2019, 12, 15)),
                Some(Rc::clone(&on_changed)),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, "2018");
            assert_eq!(selected_year.get(), Some(DateTime::utc(2018, 6, 1)));
            let root = year_picker(
                &tester,
                Some(DateTime::utc(2018, 6, 1)),
                Some(DateTime::utc(2018, 6, 9)),
                Some(DateTime::utc(2019, 12, 15)),
                Some(on_changed),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, "2019");
            assert_eq!(selected_year.get(), Some(DateTime::utc(2019, 6, 1)));
        });
    }

    #[test]
    fn year_picker_selecting_year_with_no_selected_month_uses_january() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected_year = Rc::new(Cell::new(None));
            let on_changed: ValueChanged<DateTime> = {
                let selected_year = Rc::clone(&selected_year);
                Rc::new(move |_app, date| selected_year.set(Some(date)))
            };
            let root = year_picker(
                &tester,
                None,
                Some(DateTime::utc(2018, 6, 9)),
                Some(DateTime::utc(2019, 12, 15)),
                Some(Rc::clone(&on_changed)),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, "2019");
            assert_eq!(selected_year.get(), Some(DateTime::utc(2019, 1, 1))); // january implied
            let root = year_picker(
                &tester,
                Some(DateTime::utc(2018, 1, 1)),
                Some(DateTime::utc(2018, 6, 9)),
                Some(DateTime::utc(2019, 12, 15)),
                Some(on_changed),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, "2018");
            assert_eq!(selected_year.get(), Some(DateTime::utc(2018, 6, 1)));
        });
    }

    #[test]
    fn year_picker_renders_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let picker = YearPicker::new(
                DateTime::utc(2024, 1, 1),
                DateTime::utc(2026, 1, 1),
                Some(DateTime::utc(2025, 1, 1)),
                Rc::new(|_app, _date| {}),
            );
            let theme = theme_for(&tester, None);
            tester.pump_widget(material_app(
                theme,
                crate::scaffold::Scaffold::new()
                    .body(SizedBox::shrink().child(picker))
                    .into_widget(),
            ));
        });
    }

    /// Dart's `TestCalendarDelegate extends GregorianCalendarDelegate`, overriding two methods;
    /// the rest forwards to the Gregorian delegate.
    #[derive(Debug)]
    struct TestCalendarDelegate;

    impl CalendarDelegate for TestCalendarDelegate {
        fn now(&self) -> DateTime {
            GregorianCalendarDelegate.now()
        }
        fn date_only(&self, date: &DateTime) -> DateTime {
            GregorianCalendarDelegate.date_only(date)
        }
        fn month_delta(&self, start_date: &DateTime, end_date: &DateTime) -> i32 {
            GregorianCalendarDelegate.month_delta(start_date, end_date)
        }
        fn add_months_to_month_date(&self, month_date: &DateTime, months_to_add: i32) -> DateTime {
            GregorianCalendarDelegate.add_months_to_month_date(month_date, months_to_add)
        }
        fn add_days_to_date(&self, date: &DateTime, days: i32) -> DateTime {
            GregorianCalendarDelegate.add_days_to_date(date, days)
        }
        fn first_day_offset(
            &self,
            _year: i32,
            _month: i32,
            _localizations: &dyn MaterialLocalizations,
        ) -> i32 {
            1
        }
        fn get_days_in_month(&self, _year: i32, month: i32) -> i32 {
            if month % 2 == 0 { 21 } else { 28 }
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
            GregorianCalendarDelegate.format_compact_date(date, localizations)
        }
        fn parse_compact_date(
            &self,
            input_string: Option<&str>,
            localizations: &dyn MaterialLocalizations,
        ) -> Option<DateTime> {
            GregorianCalendarDelegate.parse_compact_date(input_string, localizations)
        }
        fn date_help_text(&self, localizations: &dyn MaterialLocalizations) -> String {
            GregorianCalendarDelegate.date_help_text(localizations)
        }
    }

    /// Dart's `isA<T>()` on the delegates: the delegate's `Debug` names its type.
    fn delegate_type_name(delegate: &Rc<dyn CalendarDelegate>) -> String {
        format!("{delegate:?}")
    }

    fn pump_with_delegate(
        tester: &mut WidgetTester,
        calendar_delegate: Option<Rc<dyn CalendarDelegate>>,
    ) {
        let mut picker = CalendarDatePicker::new(
            Some(DateTime::utc(2025, 2, 26)),
            DateTime::utc(2025, 2, 1),
            DateTime::utc(2025, 5, 1),
            Rc::new(|_app, _date| {}),
        );
        if let Some(calendar_delegate) = calendar_delegate {
            picker = picker.calendar_delegate(calendar_delegate);
        }
        let theme = theme_for(tester, None);
        tester.pump_widget(material_app(
            theme,
            Material::new().child(picker).into_widget(),
        ));
    }

    fn delegates_of_the_picker_and_the_year_picker(
        tester: &mut WidgetTester,
    ) -> (Rc<dyn CalendarDelegate>, Rc<dyn CalendarDelegate>) {
        let picker = tester.find_by_type::<CalendarDatePicker>()[0];
        let calendar_picker_delegate = tester.widget(picker, |picker: &CalendarDatePicker| {
            Rc::clone(&picker.calendar_delegate)
        });
        tap_text(tester, "February 2025");
        tester.pump_and_settle();
        let year_picker = tester.find_by_type::<YearPicker>()[0];
        let year_picker_delegate = tester.widget(year_picker, |picker: &YearPicker| {
            Rc::clone(&picker.calendar_delegate)
        });
        (calendar_picker_delegate, year_picker_delegate)
    }

    #[test]
    fn calendar_delegate_defaults_to_gregorian_calendar_system() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_with_delegate(&mut tester, None);
            let (calendar_picker, year_picker) =
                delegates_of_the_picker_and_the_year_picker(&mut tester);
            assert_eq!(
                delegate_type_name(&calendar_picker),
                "GregorianCalendarDelegate"
            );
            assert_eq!(
                delegate_type_name(&year_picker),
                "GregorianCalendarDelegate"
            );
        });
    }

    #[test]
    fn calendar_delegate_using_custom_calendar_delegate_implementation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_with_delegate(&mut tester, Some(Rc::new(TestCalendarDelegate)));
            let (calendar_picker, year_picker) =
                delegates_of_the_picker_and_the_year_picker(&mut tester);
            assert_eq!(delegate_type_name(&calendar_picker), "TestCalendarDelegate");
            assert_eq!(delegate_type_name(&year_picker), "TestCalendarDelegate");
        });
    }

    use waft_painting::BorderSide;

    use waft_rendering::RenderBox as _;

    use crate::colors::Colors;
    use crate::date_picker_theme::DatePickerThemeData;
    use crate::test_support::{PaintCall, paints_calls};

    fn states_of(
        states_for_day: &Rc<RefCell<HashMap<DateTime, WidgetStates>>>,
        day: DateTime,
    ) -> WidgetStates {
        states_for_day
            .borrow()
            .get(&day)
            .cloned()
            .unwrap_or_default()
    }

    /// Dart's `buildPicker({disabled})` of 'dayBuilder preserves $state when rebuilt'.
    fn states_picker(
        states_for_day: &Rc<RefCell<HashMap<DateTime, WidgetStates>>>,
        disabled: bool,
    ) -> PickerOptions {
        let target_date = DateTime::utc(2016, 1, 12);
        PickerOptions {
            initial_date: Some(DateTime::utc(2016, 1, 15)),
            selectable_day_predicate: Some(Rc::new(move |date: DateTime| {
                !disabled || date != target_date
            })),
            day_builder: Some(custom_day_builder(Rc::clone(states_for_day))),
            ..PickerOptions::default()
        }
    }

    /// The part of 'dayBuilder preserves $state when rebuilt' that does not depend on `state`:
    /// the states survive a rebuild and a disabled day shows only the disabled state.
    fn check_states_survive_rebuild(
        tester: &mut WidgetTester,
        states_for_day: &Rc<RefCell<HashMap<DateTime, WidgetStates>>>,
        state: WidgetState,
    ) {
        let target_date = DateTime::utc(2016, 1, 12);
        tester.pump_and_settle();
        assert!(states_of(states_for_day, target_date).contains(&state));

        // Verify that interactive states reported by InkResponse survive a rebuild.
        states_for_day.borrow_mut().clear();
        pump(tester, states_picker(states_for_day, false));
        assert!(states_of(states_for_day, target_date).contains(&state));

        // Disabling the day should replace transient interactive states with disabled.
        pump(tester, states_picker(states_for_day, true));
        tester.pump_and_settle();
        assert_eq!(
            states_of(states_for_day, target_date),
            WidgetStates::from([WidgetState::Disabled])
        );
    }

    #[test]
    fn day_builder_preserves_hovered_when_rebuilt() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let states_for_day = Rc::new(RefCell::new(HashMap::new()));
            pump(&mut tester, states_picker(&states_for_day, false));
            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::new(0.0, 0.0));
            let day = text(&tester, "12");
            let at = center_of(&tester, day);
            tester.move_to(&mut mouse, at, false);
            check_states_survive_rebuild(&mut tester, &states_for_day, WidgetState::Hovered);
        });
    }

    #[test]
    fn day_builder_preserves_focused_when_rebuilt() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let states_for_day = Rc::new(RefCell::new(HashMap::new()));
            pump(&mut tester, states_picker(&states_for_day, false));
            // Focus the selected day (the 15th), then walk three days back to the 12th.
            navigate_to_the_grid(&mut tester);
            tester.pump_and_settle();
            for _ in 0..3 {
                arrow(&mut tester, TraversalDirection::Left);
                tester.pump_and_settle();
            }
            check_states_survive_rebuild(&mut tester, &states_for_day, WidgetState::Focused);
        });
    }

    #[test]
    fn day_builder_preserves_pressed_when_rebuilt() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let states_for_day = Rc::new(RefCell::new(HashMap::new()));
            pump(&mut tester, states_picker(&states_for_day, false));
            let mut touch = WidgetTester::touch(1);
            let day = text(&tester, "12");
            let at = center_of(&tester, day);
            tester.down(&mut touch, at);
            check_states_survive_rebuild(&mut tester, &states_for_day, WidgetState::Pressed);
            tester.cancel(&touch);
        });
    }

    fn today_picker_theme(border: Option<BorderSide>, foreground: Color) -> ThemeData {
        let tester = tester();
        let app = tester.cell.borrow();
        let mut picker_theme =
            DatePickerThemeData::new().today_foreground_color(<dyn WidgetStateProperty<
                Option<Color>,
            >>::all(Some(
                foreground,
            )));
        if let Some(border) = border {
            picker_theme = picker_theme.today_border(border);
        }
        ThemeData::builder()
            .date_picker_theme(picker_theme)
            .build(&app)
    }

    fn assert_today_circle(tester: &WidgetTester, color: Color) {
        let ops = tester.nearest_material_ops(text(tester, "2"));
        // The current day is painted with a circle outline.
        assert!(
            paints_calls(&ops, &[PaintCall::stroked_circle(color, 1.0)]),
            "no 1px outline of {color:?} among {} ops",
            ops.len()
        );
    }

    #[test]
    fn material2_current_date_is_highlighted() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    use_material3: Some(false),
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    current_date: DateTime::utc(2016, 1, 2),
                    ..PickerOptions::default()
                },
            );
            assert_today_circle(&tester, Color::new(0xff2196f3)); // default primary color
        });
    }

    #[test]
    fn material3_current_date_is_highlighted() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    current_date: DateTime::utc(2016, 1, 2),
                    ..PickerOptions::default()
                },
            );
            assert_today_circle(&tester, Color::new(0xff6750a4)); // default primary color
        });
    }

    fn disabled_current_date_options(use_material3: Option<bool>) -> PickerOptions {
        PickerOptions {
            use_material3,
            first_date: DateTime::utc(2016, 1, 3),
            last_date: DateTime::utc(2016, 1, 31),
            current_date: DateTime::utc(2016, 1, 2), // not between first and last
            initial_date: Some(DateTime::utc(2016, 1, 5)),
            ..PickerOptions::default()
        }
    }

    #[test]
    fn material2_current_date_is_highlighted_even_if_it_is_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(&mut tester, disabled_current_date_options(Some(false)));
            assert_today_circle(&tester, Color::new(0x61000000)); // default disabled color
        });
    }

    #[test]
    fn material3_current_date_is_highlighted_even_if_it_is_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(&mut tester, disabled_current_date_options(None));
            assert_today_circle(&tester, Color::new(0x616750a4)); // default disabled color
        });
    }

    #[test]
    fn non_null_today_border_color_should_be_respected_over_foreground_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_border_color = Colors::RED.primary();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    current_date: DateTime::utc(2016, 1, 2),
                    theme: Some(today_picker_theme(
                        Some(BorderSide {
                            color: custom_border_color,
                            ..BorderSide::default()
                        }),
                        Colors::BLUE.primary(),
                    )),
                    ..PickerOptions::default()
                },
            );
            // The current day is painted with the custom border color.
            assert_today_circle(&tester, custom_border_color);
        });
    }

    #[test]
    fn non_null_today_border_color_is_used_even_when_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_border_color = Colors::RED.primary();
            pump(
                &mut tester,
                PickerOptions {
                    theme: Some(today_picker_theme(
                        Some(BorderSide {
                            color: custom_border_color,
                            ..BorderSide::default()
                        }),
                        Colors::BLUE.primary(),
                    )),
                    ..disabled_current_date_options(None)
                },
            );
            // The current day is painted with the custom border color, not with the foreground
            // color's opacity applied, even if it is a disabled day.
            assert_today_circle(&tester, custom_border_color);
        });
    }

    #[test]
    fn transparent_today_border_should_fall_back_to_foreground_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_foreground_color = Colors::GREEN.primary();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    current_date: DateTime::utc(2016, 1, 2),
                    theme: Some(today_picker_theme(
                        Some(BorderSide {
                            color: Color::new(0x00000000),
                            ..BorderSide::default()
                        }),
                        custom_foreground_color,
                    )),
                    ..PickerOptions::default()
                },
            );
            // The current day uses the foreground color since the todayBorder color is
            // transparent.
            assert_today_circle(&tester, custom_foreground_color);
        });
    }

    fn run_initial_date_updates_are_not_reflected(use_material3: bool) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let picker_key = GlobalKey::new();
            let initial_date = DateTime::utc(2020, 1, 21);
            let updated_date = DateTime::utc(1976, 2, 23);
            let first_date = DateTime::utc(1970, 1, 1);
            let last_date = DateTime::utc(2099, 31, 12);
            let selected_color = if use_material3 {
                Color::new(0xff6750a4)
            } else {
                Color::new(0xff2196f3) // default primary color
            };
            let options = |initial_date: DateTime| PickerOptions {
                key: Some(picker_key.clone()),
                use_material3: Some(use_material3),
                initial_date: Some(initial_date),
                first_date,
                last_date,
                ..PickerOptions::default()
            };

            pump(&mut tester, options(initial_date));
            tester.pump_and_settle();

            // Month should show as January 2020.
            assert_eq!(tester.find_text("January 2020").len(), 1);
            // Selected date should be painted with a colored circle.
            let ops = tester.nearest_material_ops(text(&tester, "21"));
            assert!(paints_calls(
                &ops,
                &[PaintCall::circle(Some(selected_color))]
            ));

            // Change to the updated initialDate.
            // This should have no effect, the initialDate is only the _initial_ date.
            pump(&mut tester, options(updated_date));
            // Wait for the page scroll animation to finish.
            tester.pump_and_settle_by(Duration::from_millis(200));

            // Month should show as January 2020 still.
            assert_eq!(tester.find_text("January 2020").len(), 1);
            assert!(!has_text(&tester, "February 1976"));
            // Selected date should be painted with a colored circle.
            let ops = tester.nearest_material_ops(text(&tester, "21"));
            assert!(paints_calls(
                &ops,
                &[PaintCall::circle(Some(selected_color))]
            ));
        });
    }

    #[test]
    fn updates_to_initial_date_parameter_are_not_reflected_in_the_state_material2() {
        run_initial_date_updates_are_not_reflected(false);
    }

    #[test]
    fn updates_to_initial_date_parameter_are_not_reflected_in_the_state_material3() {
        run_initial_date_updates_are_not_reflected(true);
    }

    #[test]
    fn dragging_more_than_half_the_width_should_not_cause_a_jump() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..PickerOptions::default()
                },
            );
            tester.pump_and_settle();
            let page_view = tester.find_by_type::<waft_widgets::PageView>()[0];
            let mut gesture = WidgetTester::touch(1);
            let start = center_of(&tester, page_view);
            tester.down(&mut gesture, start);
            // This initial drag is required for the PageView to recognize the gesture, as it
            // uses DragStartBehavior.start. It does not count towards the drag distance.
            let mut move_by = |tester: &mut WidgetTester, dx: f64| {
                let at = gesture.position + waft_embedder::Offset::new(dx, 0.0);
                tester.move_to(&mut gesture, at, true);
            };
            move_by(&mut tester, 100.0);
            // Dragging for a bit less than half the width should reveal the previous month.
            move_by(&mut tester, 800.0 / 2.0 - 1.0);
            tester.pump_and_settle();
            assert_eq!(tester.find_text("January 2016").len(), 1);
            assert_eq!(tester.find_text("1").len(), 2);
            // Dragging a bit over the half should still show both.
            move_by(&mut tester, 2.0);
            tester.pump_and_settle();
            assert_eq!(tester.find_text("December 2015").len(), 1);
            assert_eq!(tester.find_text("1").len(), 2);
        });
    }

    /// Records the haptic feedback the platform is asked for: Dart's `FeedbackTester`.
    #[derive(Default)]
    struct FeedbackTester {
        haptics: RefCell<Vec<waft_embedder::HapticFeedbackType>>,
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
        let tester =
            WidgetTester::with_physical_size(platform, waft_embedder::Size::new(1600.0, 1200.0));
        tester.install_platform_fonts();
        (tester, feedback)
    }

    const HAPTIC_FEEDBACK_INTERVAL: Duration = Duration::from_millis(10);

    #[test]
    fn haptic_feedback_selecting_date_vibrates() {
        with_main_thread_stack(|| {
            let (mut tester, feedback) = feedback_tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..PickerOptions::default()
                },
            );
            for (day, count) in [("10", 1), ("12", 2), ("14", 3)] {
                tap_text(&mut tester, day);
                tester.pump(HAPTIC_FEEDBACK_INTERVAL);
                assert_eq!(feedback.haptics.borrow().len(), count, "after {day}");
            }
        });
    }

    #[test]
    fn haptic_feedback_tapping_unselectable_date_does_not_vibrate() {
        with_main_thread_stack(|| {
            let (mut tester, feedback) = feedback_tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 10)),
                    selectable_day_predicate: Some(Rc::new(|date: DateTime| date.day() % 2 == 0)),
                    ..PickerOptions::default()
                },
            );
            for day in ["11", "13", "15"] {
                tap_text(&mut tester, day);
                tester.pump(HAPTIC_FEEDBACK_INTERVAL);
                assert_eq!(feedback.haptics.borrow().len(), 0, "after {day}");
            }
        });
    }

    #[test]
    fn haptic_feedback_changing_modes_and_year_vibrates() {
        with_main_thread_stack(|| {
            let (mut tester, feedback) = feedback_tester();
            pump(
                &mut tester,
                PickerOptions {
                    initial_date: Some(DateTime::utc(2016, 1, 15)),
                    ..PickerOptions::default()
                },
            );
            tap_text(&mut tester, "January 2016");
            tester.pump(HAPTIC_FEEDBACK_INTERVAL);
            assert_eq!(feedback.haptics.borrow().len(), 1);
            tap_text(&mut tester, "2018");
            tester.pump(HAPTIC_FEEDBACK_INTERVAL);
            assert_eq!(feedback.haptics.borrow().len(), 2);
        });
    }

    #[test]
    fn default_day_selection_overlay() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = theme_for(&tester, None);
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2016, 12, 15),
                    initial_date: Some(DateTime::utc(2017, 1, 15)),
                    last_date: DateTime::utc(2017, 2, 15),
                    on_displayed_month_changed: Some(Rc::new(|_app, _date| {})),
                    theme: Some(theme.clone()),
                    ..PickerOptions::default()
                },
            );

            // `allRenderObjects.firstWhere(_RenderInkFeatures)`: the outermost Material's ink
            // features, which paint the picker's inner Material as well.
            let outer_ink_features = |tester: &WidgetTester| {
                let detector = tester.find_by_type::<waft_widgets::FocusableActionDetector>()[0];
                let app = tester.cell.borrow();
                waft_widgets::LookupBoundary::find_ancestor_render_object_of_type::<
                    crate::material::RenderInkFeatures,
                >(&app, detector)
                .expect("a Material above the picker")
            };
            let overlay_color = theme.color_scheme.on_surface_variant().with_alpha(0x14);
            let overlay = PaintCall::Circle {
                x: None,
                y: None,
                radius: Some(35.0),
                color: Some(overlay_color),
            };
            // Dart's `paintsExactlyCountTimes(#clipPath, n)`: Waft records every clip, a
            // `clipRect` as well, as a path, so this counts the clips of the size of a day's
            // highlight and leaves out the clip of the picker's page view.
            let clips = |ops: &[waft_embedder::valo::Op]| {
                ops.iter()
                    .filter(|op| match op {
                        waft_embedder::valo::Op::ClipPath { path, .. } => {
                            path.bounds().width <= 70.0
                        }
                        _ => false,
                    })
                    .count()
            };

            let ops = tester.painted_ops_deep(outer_ink_features(&tester).as_object());
            assert!(!paints_calls(&ops, std::slice::from_ref(&overlay)));
            assert_eq!(clips(&ops), 0);

            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::ZERO);
            let day = text(&tester, "25");
            let at = center_of(&tester, day);
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();
            let ops = tester.painted_ops_deep(outer_ink_features(&tester).as_object());
            assert!(paints_calls(
                &ops,
                &[PaintCall::circle(None), overlay.clone()]
            ));
            assert_eq!(clips(&ops), 1);
        });
    }

    #[test]
    fn ink_feature_paints_on_inner_material() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump(
                &mut tester,
                PickerOptions {
                    first_date: DateTime::utc(2025, 6, 1),
                    initial_date: Some(DateTime::utc(2025, 7, 20)),
                    last_date: DateTime::utc(2025, 8, 31),
                    ..PickerOptions::default()
                },
            );

            let ink_features_of = |tester: &WidgetTester, element: AnyElement| {
                let app = tester.cell.borrow();
                waft_widgets::LookupBoundary::find_ancestor_render_object_of_type::<
                    crate::material::RenderInkFeatures,
                >(&app, element)
                .expect("a Material above the element")
                .debug_ink_features(&app)
            };
            // Material outside the PageView.
            let outer = tester.find_by_type::<waft_widgets::FocusableActionDetector>()[0];
            // Material directly wrapping the PageView.
            let inner = tester.find_by_type::<waft_widgets::PageView>()[0];

            // Only the inner Material should have ink features.
            assert!(ink_features_of(&tester, outer).is_none());
            assert_eq!(ink_features_of(&tester, inner).map(|f| f.len()), Some(31));
        });
    }

    #[test]
    fn year_picker_ink_features_are_painted_on_inner_material() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = year_picker(
                &tester,
                Some(DateTime::utc(2025, 1, 1)),
                Some(DateTime::utc(2020, 1, 1)),
                Some(DateTime::utc(2030, 1, 1)),
                None,
            );
            tester.pump_widget(root);

            // The test helper's Material and the one the YearPicker adds.
            assert_eq!(tester.find_by_type::<Material>().len(), 2);

            let ink_features_of = |tester: &WidgetTester, element: AnyElement| {
                let app = tester.cell.borrow();
                waft_widgets::LookupBoundary::find_ancestor_render_object_of_type::<
                    crate::material::RenderInkFeatures,
                >(&app, element)
                .expect("a Material above the element")
            };
            // Material outside the GridView, and the one directly wrapping it.
            let outer = ink_features_of(&tester, tester.find_by_type::<YearPicker>()[0]);
            let inner =
                ink_features_of(&tester, tester.find_by_type::<waft_widgets::GridView>()[0]);
            assert_ne!(outer.as_object(), inner.as_object());
            {
                let app = tester.cell.borrow();
                assert!(outer.debug_ink_features(&app).is_none());
                assert!(inner.debug_ink_features(&app).is_none());
            }

            // Hover over the 2022 year item to trigger the ink highlight.
            let mut mouse = WidgetTester::mouse(1, 0);
            let year = text(&tester, "2022");
            let at = center_of(&tester, year);
            tester.add_pointer(&mut mouse, at);
            tester.move_to(&mut mouse, at, false);
            tester.pump(Duration::ZERO);

            // Only the inner Material should have ink features.
            let app = tester.cell.borrow();
            assert!(outer.debug_ink_features(&app).is_none());
            assert_eq!(inner.debug_ink_features(&app).map(|f| f.len()), Some(1));
        });
    }

    fn year_picker_with_today_border(
        tester: &WidgetTester,
        first_date: DateTime,
        last_date: DateTime,
        selected_date: DateTime,
        current_date: DateTime,
        theme: ThemeData,
    ) -> WidgetRef {
        let _ = tester;
        let picker = YearPicker::new(
            first_date,
            last_date,
            Some(selected_date),
            Rc::new(|_app, _date| {}),
        )
        .current_date(current_date);
        material_app(
            theme,
            Material::new()
                .child(Directionality::new(TextDirection::Ltr, picker))
                .into_widget(),
        )
    }

    /// Dart's `findYearDecoration(tester, year)`: the side color of the year's decoration.
    fn year_side_color(tester: &WidgetTester, year: &str) -> Color {
        let container = tester.ancestor_of_type::<waft_widgets::Container>(text(tester, year));
        tester.widget(container, |container: &waft_widgets::Container| {
            let decoration = container
                .decoration
                .as_deref()
                .and_then(|decoration| {
                    decoration
                        .as_any()
                        .downcast_ref::<waft_painting::ShapeDecoration>()
                })
                .expect("a ShapeDecoration");
            decoration
                .shape
                .as_outlined_border()
                .expect("an OutlinedBorder")
                .side()
                .color
        })
    }

    #[test]
    fn year_picker_non_null_today_border_color_should_be_respected_over_today_foreground_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_border_color = Colors::RED.primary();
            let theme = today_picker_theme(
                Some(BorderSide {
                    color: custom_border_color,
                    ..BorderSide::default()
                }),
                Colors::BLUE.primary(),
            );
            let root = year_picker_with_today_border(
                &tester,
                DateTime::utc(2001, 1, 1),
                DateTime::utc(2031, 12, 31),
                DateTime::utc(2016, 1, 15),
                DateTime::utc(2016, 1, 3),
                theme,
            );
            tester.pump_widget(root);
            // The current year is painted with the custom border color.
            assert_eq!(year_side_color(&tester, "2016"), custom_border_color);
        });
    }

    #[test]
    fn year_picker_non_null_today_border_color_is_used_even_when_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_border_color = Colors::RED.primary();
            let theme = today_picker_theme(
                Some(BorderSide {
                    color: custom_border_color,
                    ..BorderSide::default()
                }),
                Colors::BLUE.primary(),
            );
            let root = year_picker_with_today_border(
                &tester,
                DateTime::utc(2018, 6, 9),
                DateTime::utc(2030, 12, 15),
                DateTime::utc(2020, 1, 1),
                DateTime::utc(2016, 1, 1), // Not between first and last date.
                theme,
            );
            tester.pump_widget(root);
            // The current year is painted with the custom border color, not with the foreground
            // color's opacity applied, even if it is disabled.
            assert_eq!(year_side_color(&tester, "2016"), custom_border_color);
        });
    }

    #[test]
    fn year_picker_transparent_today_border_should_fall_back_to_foreground_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_foreground_color = Colors::GREEN.primary();
            let theme = today_picker_theme(
                Some(BorderSide {
                    color: Color::new(0x00000000),
                    ..BorderSide::default()
                }),
                custom_foreground_color,
            );
            let root = year_picker_with_today_border(
                &tester,
                DateTime::utc(2001, 1, 1),
                DateTime::utc(2031, 12, 31),
                DateTime::utc(2016, 1, 15),
                DateTime::utc(2016, 1, 3),
                theme,
            );
            tester.pump_widget(root);
            // The current year uses the foreground color since the todayBorder color is
            // transparent.
            assert_eq!(year_side_color(&tester, "2016"), custom_foreground_color);
        });
    }
}
