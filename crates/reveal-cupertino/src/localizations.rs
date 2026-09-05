//! Flutter `cupertino/localizations.dart`.

use std::rc::Rc;

use reveal_embedder::Locale;
use reveal_foundation::{App, CompleterFuture, DateTime};
use reveal_widgets::{
    BuildContext, Localizations, LocalizationsDelegate, LocalizationsDelegateRef,
};

use crate::debug::debug_check_has_cupertino_localizations;

/// Determines the order of the columns inside `CupertinoDatePicker` in
/// time and date time mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatePickerDateTimeOrder {
    /// Order of the columns, from left to right: date, hour, minute, am/pm.
    ///
    /// Example: Fri Aug 31 | 02 | 44 | PM.
    DateTimeDayPeriod,
    /// Order of the columns, from left to right: date, am/pm, hour, minute.
    ///
    /// Example: Fri Aug 31 | PM | 02 | 44.
    DateDayPeriodTime,
    /// Order of the columns, from left to right: hour, minute, am/pm, date.
    ///
    /// Example: 02 | 44 | PM | Fri Aug 31.
    TimeDayPeriodDate,
    /// Order of the columns, from left to right: am/pm, hour, minute, date.
    ///
    /// Example: PM | 02 | 44 | Fri Aug 31.
    DayPeriodTimeDate,
}

/// Determines the order of the columns inside `CupertinoDatePicker` in date mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatePickerDateOrder {
    /// Order of the columns, from left to right: day, month, year.
    ///
    /// Example: 12 | March | 1996.
    Dmy,
    /// Order of the columns, from left to right: month, day, year.
    ///
    /// Example: March | 12 | 1996.
    Mdy,
    /// Order of the columns, from left to right: year, month, day.
    ///
    /// Example: 1996 | March | 12.
    Ymd,
    /// Order of the columns, from left to right: year, day, month.
    ///
    /// Example: 1996 | 12 | March.
    Ydm,
}

/// Defines the localized resource values used by the Cupertino widgets.
///
/// See also:
///
///  * [`DefaultCupertinoLocalizations`], the default, English-only, implementation
///    of this interface.
pub trait CupertinoLocalizations: 'static {
    /// Year that is shown in `CupertinoDatePicker` spinner corresponding to the
    /// given year index.
    ///
    /// Examples: date_picker_year(1) in:
    ///
    ///  - US English: 2018
    ///  - Korean: 2018년
    fn date_picker_year(&self, year_index: i32) -> String;

    /// Month that is shown in `CupertinoDatePicker` spinner corresponding to
    /// the given month index.
    ///
    /// Examples: date_picker_month(1) in:
    ///
    ///  - US English: January
    ///  - Korean: 1월
    ///  - Russian: января
    fn date_picker_month(&self, month_index: i32) -> String;

    /// Month that is shown in `CupertinoDatePicker` spinner corresponding to
    /// the given month index in `CupertinoDatePickerMode.monthYear` mode.
    ///
    /// Examples: date_picker_standalone_month(1) in:
    ///
    ///  - US English: January
    ///  - Korean: 1월
    ///  - Russian: Январь
    fn date_picker_standalone_month(&self, month_index: i32) -> String;

    /// Day of month that is shown in `CupertinoDatePicker` spinner corresponding
    /// to the given day index.
    ///
    /// If `week_day` is provided then it will also show weekday name alongside the numerical day.
    ///
    /// Examples: date_picker_day_of_month(1) in:
    ///
    ///  - US English: 1
    ///  - Korean: 1일
    ///
    /// Examples: date_picker_day_of_month(1, 1) in:
    ///
    ///  - US English: Mon 1
    fn date_picker_day_of_month(&self, day_index: i32, week_day: Option<i32>) -> String;

    /// The medium-width date format that is shown in `CupertinoDatePicker`
    /// spinner. Abbreviates month and days of week.
    ///
    /// Examples:
    ///
    /// - US English: Wed Sep 27
    /// - Russian: ср сент. 27
    fn date_picker_medium_date(&self, date: &DateTime) -> String;

    /// Hour that is shown in `CupertinoDatePicker` spinner corresponding
    /// to the given hour value.
    ///
    /// Examples: date_picker_hour(1) in:
    ///
    ///  - US English: 1
    ///  - Arabic: ٠١
    fn date_picker_hour(&self, hour: i32) -> String;

    /// Semantics label for the given hour value in `CupertinoDatePicker`.
    fn date_picker_hour_semantics_label(&self, hour: i32) -> Option<String>;

    /// Minute that is shown in `CupertinoDatePicker` spinner corresponding
    /// to the given minute value.
    ///
    /// Examples: date_picker_minute(1) in:
    ///
    ///  - US English: 01
    ///  - Arabic: ٠١
    fn date_picker_minute(&self, minute: i32) -> String;

    /// Semantics label for the given minute value in `CupertinoDatePicker`.
    fn date_picker_minute_semantics_label(&self, minute: i32) -> Option<String>;

    /// The order of the date elements that will be shown in `CupertinoDatePicker`.
    fn date_picker_date_order(&self) -> DatePickerDateOrder;

    /// The order of the time elements that will be shown in `CupertinoDatePicker`.
    fn date_picker_date_time_order(&self) -> DatePickerDateTimeOrder;

    /// The abbreviation for ante meridiem (before noon) shown in the time picker.
    fn ante_meridiem_abbreviation(&self) -> String;

    /// The abbreviation for post meridiem (after noon) shown in the time picker.
    fn post_meridiem_abbreviation(&self) -> String;

    /// The term used by the system to announce dialog alerts.
    fn today_label(&self) -> String;

    /// The term used by the system to announce dialog alerts.
    fn alert_dialog_label(&self) -> String;

    /// The accessibility label used on a tab in a `CupertinoTabBar`.
    ///
    /// This message describes the index of the selected tab and how many tabs
    /// there are, e.g. 'tab, 1 of 2' in United States English.
    ///
    /// `tab_index` and `tab_count` must be greater than or equal to one.
    fn tab_semantics_label(&self, tab_index: i32, tab_count: i32) -> String;

    /// Hour that is shown in `CupertinoTimerPicker` corresponding to
    /// the given hour value.
    ///
    /// Examples: timer_picker_hour(1) in:
    ///
    ///  - US English: 1
    ///  - Arabic: ١
    fn timer_picker_hour(&self, hour: i32) -> String;

    /// Minute that is shown in `CupertinoTimerPicker` corresponding to
    /// the given minute value.
    ///
    /// Examples: timer_picker_minute(1) in:
    ///
    ///  - US English: 1
    ///  - Arabic: ١
    fn timer_picker_minute(&self, minute: i32) -> String;

    /// Second that is shown in `CupertinoTimerPicker` corresponding to
    /// the given second value.
    ///
    /// Examples: timer_picker_second(1) in:
    ///
    ///  - US English: 1
    ///  - Arabic: ١
    fn timer_picker_second(&self, second: i32) -> String;

    /// Label that appears next to the hour picker in
    /// `CupertinoTimerPicker` when selected hour value is `hour`.
    /// This function will deal with pluralization based on the `hour` parameter.
    fn timer_picker_hour_label(&self, hour: i32) -> Option<String>;

    /// All possible hour labels that appears next to the hour picker in
    /// `CupertinoTimerPicker`.
    fn timer_picker_hour_labels(&self) -> Vec<String>;

    /// Label that appears next to the minute picker in
    /// `CupertinoTimerPicker` when selected minute value is `minute`.
    /// This function will deal with pluralization based on the `minute` parameter.
    fn timer_picker_minute_label(&self, minute: i32) -> Option<String>;

    /// All possible minute labels that appears next to the minute picker in
    /// `CupertinoTimerPicker`.
    fn timer_picker_minute_labels(&self) -> Vec<String>;

    /// Label that appears next to the minute picker in
    /// `CupertinoTimerPicker` when selected minute value is `second`.
    /// This function will deal with pluralization based on the `second` parameter.
    fn timer_picker_second_label(&self, second: i32) -> Option<String>;

    /// All possible second labels that appears next to the second picker in
    /// `CupertinoTimerPicker`.
    fn timer_picker_second_labels(&self) -> Vec<String>;

    /// The term used for cutting.
    fn cut_button_label(&self) -> String;

    /// The term used for copying.
    fn copy_button_label(&self) -> String;

    /// The term used for pasting.
    fn paste_button_label(&self) -> String;

    /// The term used for clearing a field.
    fn clear_button_label(&self) -> String;

    /// Label that appears in the Cupertino toolbar when the spell checker
    /// couldn't find any replacements for the current word.
    fn no_spell_check_replacements_label(&self) -> String;

    /// The term used for selecting everything.
    fn select_all_button_label(&self) -> String;

    /// The term used for looking up a definition.
    fn look_up_button_label(&self) -> String;

    /// The term used for launching a web search on a selection.
    fn search_web_button_label(&self) -> String;

    /// The term used for launching a share dialog for a selection.
    fn share_button_label(&self) -> String;

    /// The default placeholder used in `CupertinoSearchTextField`.
    fn search_text_field_placeholder_label(&self) -> String;

    /// Label read out by accessibility tools (VoiceOver) for a modal
    /// barrier to indicate that a tap dismisses the barrier.
    ///
    /// A modal barrier can for example be found behind an alert or popup menu.
    fn modal_barrier_dismiss_label(&self) -> String;

    /// Label read out by accessibility tools (VoiceOver) for a context menu to
    /// indicate that a tap outside dismisses the context menu.
    fn menu_dismiss_label(&self) -> String;

    /// The term used for cancelling.
    fn cancel_button_label(&self) -> String;

    /// The default label for the back button in the Cupertino navigation bar.
    fn back_button_label(&self) -> String;

    /// The semantics hint to describe the tap action on an expanded
    /// `CupertinoExpansionTile`.
    fn expansion_tile_expanded_hint(&self) -> String {
        String::from("double tap to collapse")
    }

    /// The semantics hint to describe the tap action on a collapsed
    /// `CupertinoExpansionTile`.
    fn expansion_tile_collapsed_hint(&self) -> String {
        String::from("double tap to expand")
    }

    /// The semantics hint to describe the tap action on an expanded
    /// `CupertinoExpansionTile` on iOS and macOS.
    fn expansion_tile_expanded_tap_hint(&self) -> String {
        String::from("Collapse")
    }

    /// The semantics hint to describe the tap action on a collapsed
    /// `CupertinoExpansionTile` on iOS and macOS.
    fn expansion_tile_collapsed_tap_hint(&self) -> String {
        String::from("Expand for more details")
    }

    /// The label for the `CupertinoExpansionTile` when it is expanded.
    fn expanded_hint(&self) -> String {
        String::from("Collapsed")
    }

    /// The label for the `CupertinoExpansionTile` when it is collapsed.
    fn collapsed_hint(&self) -> String {
        String::from("Expanded")
    }
}

impl dyn CupertinoLocalizations {
    /// The `CupertinoLocalizations` from the closest [`Localizations`] instance
    /// that encloses the given context.
    ///
    /// If no [`CupertinoLocalizations`] are available in the given `context`, this
    /// method throws an exception.
    ///
    /// This method is just a convenient shorthand for:
    /// `Localizations::of::<dyn CupertinoLocalizations>(app, context)`.
    ///
    /// References to the localized resources defined by this class are typically
    /// written in terms of this method. For example:
    ///
    /// ```text
    /// <dyn CupertinoLocalizations>::of(app, context).anteMeridiemAbbreviation()
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> Rc<dyn CupertinoLocalizations> {
        debug_assert!(debug_check_has_cupertino_localizations(app, context));
        Localizations::of::<dyn CupertinoLocalizations>(app, context)
            .expect("a CupertinoLocalizations ancestor")
    }
}

#[derive(Debug)]
struct CupertinoLocalizationsDelegate;

impl LocalizationsDelegate<dyn CupertinoLocalizations> for CupertinoLocalizationsDelegate {
    fn is_supported(&self, locale: &Locale) -> bool {
        locale.language_code == "en"
    }

    fn load(
        &self,
        _app: &mut App,
        locale: &Locale,
    ) -> CompleterFuture<Rc<dyn CupertinoLocalizations>> {
        DefaultCupertinoLocalizations::load(locale)
    }

    fn should_reload(&self, _old: &Self) -> bool {
        false
    }
}

/// US English strings for the Cupertino widgets.
#[derive(Debug, Default)]
pub struct DefaultCupertinoLocalizations;

const SHORT_WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl DefaultCupertinoLocalizations {
    /// Constructs an object that defines the cupertino widgets' localized strings
    /// for US English (only).
    ///
    /// [`delegate`](Self::delegate) is the delegate.
    pub const fn new() -> DefaultCupertinoLocalizations {
        DefaultCupertinoLocalizations
    }

    /// Creates an object that provides US English resource values for the
    /// cupertino library widgets.
    ///
    /// The `locale` parameter is ignored.
    ///
    /// This method is typically used to create a [`LocalizationsDelegate`].
    pub fn load(_locale: &Locale) -> CompleterFuture<Rc<dyn CupertinoLocalizations>> {
        CompleterFuture::ready(Rc::new(DefaultCupertinoLocalizations::new()))
    }

    /// A [`LocalizationsDelegate`] that uses [`DefaultCupertinoLocalizations::load`]
    /// to create an instance of this class.
    pub fn delegate() -> LocalizationsDelegateRef {
        LocalizationsDelegateRef::new(CupertinoLocalizationsDelegate)
    }
}

fn month_name(table: &[&str; 12], month_index: i32) -> String {
    let index = usize::try_from(month_index - DateTime::JANUARY).expect("a month in 1..=12");
    String::from(table[index])
}

fn short_weekday(week_day: i32) -> &'static str {
    let index = usize::try_from(week_day - DateTime::MONDAY).expect("a weekday in 1..=7");
    SHORT_WEEKDAYS[index]
}

impl CupertinoLocalizations for DefaultCupertinoLocalizations {
    fn date_picker_year(&self, year_index: i32) -> String {
        year_index.to_string()
    }

    fn date_picker_month(&self, month_index: i32) -> String {
        month_name(&MONTHS, month_index)
    }

    fn date_picker_standalone_month(&self, month_index: i32) -> String {
        month_name(&MONTHS, month_index)
    }

    fn date_picker_day_of_month(&self, day_index: i32, week_day: Option<i32>) -> String {
        match week_day {
            Some(week_day) => format!(" {} {day_index} ", short_weekday(week_day)),
            None => day_index.to_string(),
        }
    }

    fn date_picker_hour(&self, hour: i32) -> String {
        hour.to_string()
    }

    fn date_picker_hour_semantics_label(&self, hour: i32) -> Option<String> {
        Some(format!("{hour} o'clock"))
    }

    fn date_picker_minute(&self, minute: i32) -> String {
        format!("{minute:02}")
    }

    fn date_picker_minute_semantics_label(&self, minute: i32) -> Option<String> {
        if minute == 1 {
            return Some(String::from("1 minute"));
        }
        Some(format!("{minute} minutes"))
    }

    fn date_picker_medium_date(&self, date: &DateTime) -> String {
        format!(
            "{} {} {:<2}",
            short_weekday(date.weekday()),
            month_name(&SHORT_MONTHS, date.month()),
            date.day()
        )
    }

    fn date_picker_date_order(&self) -> DatePickerDateOrder {
        DatePickerDateOrder::Mdy
    }

    fn date_picker_date_time_order(&self) -> DatePickerDateTimeOrder {
        DatePickerDateTimeOrder::DateTimeDayPeriod
    }

    fn ante_meridiem_abbreviation(&self) -> String {
        String::from("AM")
    }

    fn post_meridiem_abbreviation(&self) -> String {
        String::from("PM")
    }

    fn today_label(&self) -> String {
        String::from("Today")
    }

    fn alert_dialog_label(&self) -> String {
        String::from("Alert")
    }

    fn tab_semantics_label(&self, tab_index: i32, tab_count: i32) -> String {
        debug_assert!(tab_index >= 1);
        debug_assert!(tab_count >= 1);
        format!("Tab {tab_index} of {tab_count}")
    }

    fn timer_picker_hour(&self, hour: i32) -> String {
        hour.to_string()
    }

    fn timer_picker_minute(&self, minute: i32) -> String {
        minute.to_string()
    }

    fn timer_picker_second(&self, second: i32) -> String {
        second.to_string()
    }

    fn timer_picker_hour_label(&self, hour: i32) -> Option<String> {
        Some(String::from(if hour == 1 { "hour" } else { "hours" }))
    }

    fn timer_picker_hour_labels(&self) -> Vec<String> {
        vec![String::from("hour"), String::from("hours")]
    }

    fn timer_picker_minute_label(&self, _minute: i32) -> Option<String> {
        Some(String::from("min."))
    }

    fn timer_picker_minute_labels(&self) -> Vec<String> {
        vec![String::from("min.")]
    }

    fn timer_picker_second_label(&self, _second: i32) -> Option<String> {
        Some(String::from("sec."))
    }

    fn timer_picker_second_labels(&self) -> Vec<String> {
        vec![String::from("sec.")]
    }

    fn cut_button_label(&self) -> String {
        String::from("Cut")
    }

    fn copy_button_label(&self) -> String {
        String::from("Copy")
    }

    fn paste_button_label(&self) -> String {
        String::from("Paste")
    }

    fn clear_button_label(&self) -> String {
        String::from("Clear")
    }

    fn no_spell_check_replacements_label(&self) -> String {
        String::from("No Replacements Found")
    }

    fn select_all_button_label(&self) -> String {
        String::from("Select All")
    }

    fn look_up_button_label(&self) -> String {
        String::from("Look Up")
    }

    fn search_web_button_label(&self) -> String {
        String::from("Search Web")
    }

    fn share_button_label(&self) -> String {
        String::from("Share...")
    }

    fn search_text_field_placeholder_label(&self) -> String {
        String::from("Search")
    }

    fn modal_barrier_dismiss_label(&self) -> String {
        String::from("Dismiss")
    }

    fn menu_dismiss_label(&self) -> String {
        String::from("Dismiss menu")
    }

    fn cancel_button_label(&self) -> String {
        String::from("Cancel")
    }

    fn back_button_label(&self) -> String {
        String::from("Back")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use reveal_widgets::{Builder, DefaultWidgetsLocalizations, IntoWidget, SizedBox};

    use super::*;
    use crate::test_support::{build, test_cell};

    #[test]
    fn the_default_strings_format_dates_like_dart() {
        let strings = DefaultCupertinoLocalizations::new();
        assert_eq!(
            strings.date_picker_medium_date(&DateTime::utc(2026, 9, 5)),
            "Sat Sep 5 "
        );
        assert_eq!(
            strings.date_picker_medium_date(&DateTime::utc(2026, 12, 25)),
            "Fri Dec 25"
        );
        assert_eq!(
            strings.date_picker_day_of_month(5, Some(DateTime::SATURDAY)),
            " Sat 5 "
        );
        assert_eq!(strings.date_picker_day_of_month(5, None), "5");
        assert_eq!(strings.date_picker_month(DateTime::FEBRUARY), "February");
        assert_eq!(strings.date_picker_minute(7), "07");
        assert_eq!(
            strings.date_picker_minute_semantics_label(1).as_deref(),
            Some("1 minute")
        );
        assert_eq!(strings.tab_semantics_label(1, 3), "Tab 1 of 3");
        assert_eq!(strings.timer_picker_hour_label(2).as_deref(), Some("hours"));
        assert_eq!(
            strings.expansion_tile_expanded_hint(),
            "double tap to collapse"
        );
    }

    #[test]
    fn the_delegate_supports_english_only() {
        let delegate = DefaultCupertinoLocalizations::delegate();
        assert!(delegate.is_supported(&Locale::new("en").country_code("GB")));
        assert!(!delegate.is_supported(&Locale::new("fr")));
    }

    #[test]
    fn load_completes_synchronously() {
        let strings = DefaultCupertinoLocalizations::load(&Locale::new("en"));
        assert!(strings.is_completed(), "Dart's SynchronousFuture");
        assert_eq!(
            strings.peek().expect("complete").back_button_label(),
            "Back"
        );
    }

    #[test]
    fn of_finds_the_strings_through_a_localizations_ancestor() {
        let cell = test_cell();
        let app = cell.borrow();
        let seen = Rc::new(RefCell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() =
                    Some(<dyn CupertinoLocalizations>::of(app, context).alert_dialog_label());
                SizedBox::shrink().into_widget()
            }
        });
        drop(app);
        build(
            &cell,
            Localizations::new(
                Locale::new("en").country_code("US"),
                vec![
                    DefaultWidgetsLocalizations::delegate(),
                    DefaultCupertinoLocalizations::delegate(),
                ],
            )
            .child(probe)
            .into_widget(),
        );
        assert_eq!(seen.borrow().as_deref(), Some("Alert"));
    }
}
