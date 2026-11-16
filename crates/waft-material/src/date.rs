//! Flutter counterpart: material_ui's `date.dart`.
//!
//! The calendar arithmetic the date pickers share: [`DateUtils`], the [`CalendarDelegate`] a
//! picker asks for dates, and the two modes and the range value the pickers pass around. Dates
//! are local [`DateTime`](waft_foundation::DateTime)s, as Dart's `DateTime(year, month, day)`
//! makes them, so a "day" here is a calendar day in the local time zone, and what makes one takes
//! the [`App`] whose platform gives the zone.

use std::fmt;
use std::time::Duration;

use waft_foundation::{App, DateTime};
use waft_services::TextInputFormatterRef;

use crate::material_localizations::MaterialLocalizations;

/// A delegate that handles all of the calendar operations a date picker needs: formatting,
/// parsing and the arithmetic on months and days.
///
/// Dart's `CalendarDelegate<T extends DateTime>` is generic over a subclass of `DateTime`.
/// Rust has no subclasses of [`DateTime`], so a delegate here always works on [`DateTime`]
/// values; a non-Gregorian calendar maps its dates onto them.
pub trait CalendarDelegate: fmt::Debug {
    /// Returns a [`DateTime`] representing the current moment.
    fn now(&self, app: &App) -> DateTime;

    /// Returns a [`DateTime`] with the date of the original, but time set to midnight.
    fn date_only(&self, app: &App, date: &DateTime) -> DateTime;

    /// Returns a [`DateTimeRange`] with the dates of the original, but times set to midnight.
    fn dates_only(&self, app: &App, range: &DateTimeRange) -> DateTimeRange {
        DateTimeRange::new(
            self.date_only(app, &range.start),
            self.date_only(app, &range.end),
        )
    }

    /// Returns true if the two [`DateTime`] objects have the same day, month, and year, or are
    /// both null.
    fn is_same_day(&self, date_a: Option<&DateTime>, date_b: Option<&DateTime>) -> bool {
        date_a.map(|date| (date.year(), date.month(), date.day()))
            == date_b.map(|date| (date.year(), date.month(), date.day()))
    }

    /// Returns true if the two [`DateTime`] objects have the same month and year, or are both
    /// null.
    fn is_same_month(&self, date_a: Option<&DateTime>, date_b: Option<&DateTime>) -> bool {
        date_a.map(|date| (date.year(), date.month()))
            == date_b.map(|date| (date.year(), date.month()))
    }

    /// Determines the number of months between two [`DateTime`] objects.
    fn month_delta(&self, start_date: &DateTime, end_date: &DateTime) -> i32;

    /// Returns a [`DateTime`] with the added number of months and the day set to 1 and time
    /// set to midnight.
    fn add_months_to_month_date(
        &self,
        app: &App,
        month_date: &DateTime,
        months_to_add: i32,
    ) -> DateTime;

    /// Returns a [`DateTime`] with the added number of days and time set to midnight.
    fn add_days_to_date(&self, app: &App, date: &DateTime, days: i32) -> DateTime;

    /// Computes the offset from the first day of the week that the first day of the `month`
    /// falls on.
    fn first_day_offset(
        &self,
        year: i32,
        month: i32,
        localizations: &dyn MaterialLocalizations,
    ) -> i32;

    /// Returns the number of days in a month, according to the proleptic Gregorian calendar.
    fn get_days_in_month(&self, year: i32, month: i32) -> i32;

    /// Returns a [`DateTime`] with the given `year` and `month`, on the first day of the month.
    fn get_month(&self, app: &App, year: i32, month: i32) -> DateTime;

    /// Returns a [`DateTime`] with the given `year`, `month` and `day`.
    fn get_day(&self, app: &App, year: i32, month: i32, day: i32) -> DateTime;

    /// Formats the month and the year of the given [`DateTime`].
    fn format_month_year(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String;

    /// Formats the given `year` as a year.
    fn format_year(
        &self,
        app: &App,
        year: i32,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        localizations.format_year(&DateTime::new(app, year, 1, 1))
    }

    /// Formats the given [`DateTime`] as a medium-length date.
    fn format_medium_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String;

    /// Formats the month and day of the given [`DateTime`], in a short form.
    fn format_short_month_day(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String;

    /// Formats the given [`DateTime`] as a short date.
    fn format_short_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String;

    /// Formats the given [`DateTime`] as a full date.
    fn format_full_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String;

    /// Formats the given [`DateTime`] as a compact date.
    fn format_compact_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String;

    /// Parses the given input string as a compact date.
    fn parse_compact_date(
        &self,
        app: &App,
        input_string: Option<&str>,
        localizations: &dyn MaterialLocalizations,
    ) -> Option<DateTime>;

    /// The help text displayed in the input date picker.
    fn date_help_text(&self, localizations: &dyn MaterialLocalizations) -> String;

    /// The formatters that restrict what a user may type when entering dates by keyboard.
    fn keyboard_input_formatters(
        &self,
        _localizations: &dyn MaterialLocalizations,
    ) -> Option<Vec<TextInputFormatterRef>> {
        None
    }
}

/// A [`CalendarDelegate`] for the Gregorian calendar, which is what [`DateUtils`] computes in.
#[derive(Clone, Copy, Debug, Default)]
pub struct GregorianCalendarDelegate;

impl CalendarDelegate for GregorianCalendarDelegate {
    fn now(&self, app: &App) -> DateTime {
        DateTime::now(app)
    }

    fn date_only(&self, app: &App, date: &DateTime) -> DateTime {
        DateUtils::date_only(app, date)
    }

    fn month_delta(&self, start_date: &DateTime, end_date: &DateTime) -> i32 {
        DateUtils::month_delta(start_date, end_date)
    }

    fn add_months_to_month_date(
        &self,
        app: &App,
        month_date: &DateTime,
        months_to_add: i32,
    ) -> DateTime {
        DateUtils::add_months_to_month_date(app, month_date, months_to_add)
    }

    fn add_days_to_date(&self, app: &App, date: &DateTime, days: i32) -> DateTime {
        DateUtils::add_days_to_date(app, date, days)
    }

    fn first_day_offset(
        &self,
        year: i32,
        month: i32,
        localizations: &dyn MaterialLocalizations,
    ) -> i32 {
        DateUtils::first_day_offset(year, month, localizations)
    }

    fn get_days_in_month(&self, year: i32, month: i32) -> i32 {
        DateUtils::get_days_in_month(year, month)
    }

    fn get_month(&self, app: &App, year: i32, month: i32) -> DateTime {
        DateTime::new(app, year, month, 1)
    }

    fn get_day(&self, app: &App, year: i32, month: i32, day: i32) -> DateTime {
        DateTime::new(app, year, month, day)
    }

    fn format_month_year(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        localizations.format_month_year(date)
    }

    fn format_medium_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        localizations.format_medium_date(date)
    }

    fn format_short_month_day(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        localizations.format_short_month_day(date)
    }

    fn format_short_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        localizations.format_short_date(date)
    }

    fn format_full_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        localizations.format_full_date(date)
    }

    fn format_compact_date(
        &self,
        date: &DateTime,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        localizations.format_compact_date(date)
    }

    fn parse_compact_date(
        &self,
        app: &App,
        input_string: Option<&str>,
        localizations: &dyn MaterialLocalizations,
    ) -> Option<DateTime> {
        localizations.parse_compact_date(app, input_string)
    }

    fn date_help_text(&self, localizations: &dyn MaterialLocalizations) -> String {
        localizations.date_help_text()
    }
}

/// Utility functions for working with dates.
pub struct DateUtils;

impl DateUtils {
    /// Returns a [`DateTime`] with the date of the original, but time set to midnight.
    pub fn date_only(app: &App, date: &DateTime) -> DateTime {
        DateTime::new(app, date.year(), date.month(), date.day())
    }

    /// Returns a [`DateTimeRange`] with the dates of the original, but with times set to
    /// midnight.
    ///
    /// See also: [`date_only`](Self::date_only), which does the same thing for a single date.
    pub fn dates_only(app: &App, range: &DateTimeRange) -> DateTimeRange {
        DateTimeRange::new(
            Self::date_only(app, &range.start),
            Self::date_only(app, &range.end),
        )
    }

    /// Returns true if the two [`DateTime`] objects have the same day, month, and year, or are
    /// both null.
    pub fn is_same_day(date_a: Option<&DateTime>, date_b: Option<&DateTime>) -> bool {
        GregorianCalendarDelegate.is_same_day(date_a, date_b)
    }

    /// Returns true if the two [`DateTime`] objects have the same month and year, or are both
    /// null.
    pub fn is_same_month(date_a: Option<&DateTime>, date_b: Option<&DateTime>) -> bool {
        GregorianCalendarDelegate.is_same_month(date_a, date_b)
    }

    /// Determines the number of months between two [`DateTime`] objects.
    ///
    /// For example:
    ///
    /// ```
    /// # use waft_foundation::{AppCell, DateTime};
    /// # use waft_material::DateUtils;
    /// # let cell = AppCell::new();
    /// # let app = cell.borrow();
    /// let date1 = DateTime::new(&app, 2019, 6, 15);
    /// let date2 = DateTime::new(&app, 2020, 1, 15);
    /// let difference = DateUtils::month_delta(&date1, &date2);
    /// assert_eq!(difference, 7);
    /// ```
    ///
    /// The days are not taken into account, so the difference is always an integer number of
    /// months.
    pub fn month_delta(start_date: &DateTime, end_date: &DateTime) -> i32 {
        (end_date.year() - start_date.year()) * 12 + end_date.month() - start_date.month()
    }

    /// Returns a [`DateTime`] that is `month_date` with the added number of months and the day
    /// set to 1 and time set to midnight.
    ///
    /// For example:
    ///
    /// ```
    /// # use waft_foundation::{AppCell, DateTime};
    /// # use waft_material::DateUtils;
    /// # let cell = AppCell::new();
    /// # let app = cell.borrow();
    /// let date = DateTime::new(&app, 2019, 1, 15);
    /// let next_month = DateUtils::add_months_to_month_date(&app, &date, 1);
    /// assert_eq!(next_month, DateTime::new(&app, 2019, 2, 1));
    /// ```
    ///
    /// `month_date` is assumed to already be on the first of the month.
    pub fn add_months_to_month_date(
        app: &App,
        month_date: &DateTime,
        months_to_add: i32,
    ) -> DateTime {
        DateTime::new(
            app,
            month_date.year(),
            month_date.month() + months_to_add,
            1,
        )
    }

    /// Returns a [`DateTime`] with the added number of days and time set to midnight.
    pub fn add_days_to_date(app: &App, date: &DateTime, days: i32) -> DateTime {
        DateTime::new(app, date.year(), date.month(), date.day() + days)
    }

    /// Computes the offset from the first day of the week that the first day of the `month`
    /// falls on.
    ///
    /// For example, September 1, 2017 falls on a Friday, which in the calendar localized for
    /// United States English appears as:
    ///
    /// ```text
    /// S M T W T F S
    /// _ _ _ _ _ 1 2
    /// ```
    ///
    /// The offset for the first day of the months is the number of leading blanks in the
    /// calendar, i.e. 5.
    ///
    /// The same date localized for the Russian calendar has a different offset, because the
    /// first day of week is Monday rather than Sunday:
    ///
    /// ```text
    /// M T W T F S S
    /// _ _ _ _ 1 2 3
    /// ```
    ///
    /// So the offset is 4, rather than 5.
    ///
    /// This code consolidates the following:
    ///
    /// - [`DateTime::weekday`] provides a 1-based index into days of week, with 1 falling on
    ///   Monday.
    /// - [`MaterialLocalizations::first_day_of_week_index`] provides a 0-based index into the
    ///   [`MaterialLocalizations::narrow_weekdays`] list.
    /// - [`MaterialLocalizations::narrow_weekdays`] list provides localized names of days of
    ///   week, always starting with Sunday and ending with Saturday.
    pub fn first_day_offset(
        year: i32,
        month: i32,
        localizations: &dyn MaterialLocalizations,
    ) -> i32 {
        // 0-based day of week for the month and year, with 0 representing Monday. Dart reads it
        // from the local `DateTime(year, month)`; a calendar date's weekday is the same in every
        // zone, so the UTC date gives it without asking the platform.
        let weekday_from_monday = DateTime::utc(year, month, 1).weekday() - 1;

        // 0-based start of week depending on the locale, with 0 representing Sunday.
        let first_day_of_week_index = localizations.first_day_of_week_index();

        // firstDayOfWeekIndex recomputed to be Monday-based, in order to compare with
        // weekdayFromMonday.
        let first_day_of_week_index = (first_day_of_week_index - 1).rem_euclid(7);

        // Number of days between the first day of week appearing on the calendar, and the day
        // corresponding to the first of the month.
        (weekday_from_monday - first_day_of_week_index).rem_euclid(7)
    }

    /// Returns the number of days in a month, according to the proleptic Gregorian calendar.
    ///
    /// This applies the leap year logic introduced by the Gregorian reforms of 1582. It will
    /// not give valid results for dates prior to that time.
    pub fn get_days_in_month(year: i32, month: i32) -> i32 {
        if month == DateTime::FEBRUARY {
            let is_leap_year = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            return if is_leap_year { 29 } else { 28 };
        }
        const DAYS_IN_MONTH: [i32; 12] = [31, -1, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        DAYS_IN_MONTH[(month - 1) as usize]
    }
}

/// Whether `a`'s calendar day comes before `b`'s: how the pickers' constructor checks compare
/// the days [`CalendarDelegate::date_only`] gives, which they cannot make without the [`App`]
/// (`into_widget` has none). Local midnights are in the order of their dates, so the dates'
/// fields order them.
pub(crate) fn is_day_before(a: &DateTime, b: &DateTime) -> bool {
    (a.year(), a.month(), a.day()) < (b.year(), b.month(), b.day())
}

/// Initial display mode of the date picker dialog.
///
/// Date picker UI mode for either showing a list of available years or a monthly calendar
/// initially in the dialog shown by calling [`showDatePicker`](crate::show_date_picker).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatePickerEntryMode {
    /// User picks a date from calendar grid. Can switch to [`Input`](Self::Input) by pressing
    /// a button in the dialog.
    Calendar,

    /// User can input the date with a text field. Can switch to [`Calendar`](Self::Calendar)
    /// by pressing a button in the dialog.
    Input,

    /// User can only pick a date from calendar grid. Switching to
    /// [`Input`](Self::Input) is disabled.
    CalendarOnly,

    /// User can only input the date with a text field. Switching to
    /// [`Calendar`](Self::Calendar) is disabled.
    InputOnly,
}

/// Initial display of a calendar date picker.
///
/// Either a grid of available years or a monthly calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DatePickerMode {
    /// Choosing a month and day.
    Day,

    /// Choosing a year.
    Year,
}

/// Represents a range of dates, i.e. the span of time from [`start`](Self::start) to
/// [`end`](Self::end).
///
/// The range includes the start and end dates, so with `start` and `end` on the same day the
/// range covers one day.
///
/// Dart's `DateTimeRange<T extends DateTime>` is generic over a subclass of `DateTime`;
/// see [`CalendarDelegate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateTimeRange {
    /// The start of the range of dates.
    pub start: DateTime,

    /// The end of the range of dates.
    pub end: DateTime,
}

impl DateTimeRange {
    /// Creates a date range for the given start and end [`DateTime`].
    ///
    /// Dart's `DateTimeRange({required start, required end})`.
    pub fn new(start: DateTime, end: DateTime) -> DateTimeRange {
        debug_assert!(!start.is_after(&end), "start must not be after end");
        DateTimeRange { start, end }
    }

    /// Returns a [`Duration`] of the time between [`start`](Self::start) and
    /// [`end`](Self::end).
    ///
    /// See [`DateTime::difference`] in Dart; here the difference of the two instants.
    pub fn duration(&self) -> Duration {
        Duration::from_micros(
            (self.end.microseconds_since_epoch() - self.start.microseconds_since_epoch()) as u64,
        )
    }
}

impl fmt::Display for DateTimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use waft_foundation::AppCell;

    use super::*;
    use crate::material_localizations::DefaultMaterialLocalizations;

    /// Not in Flutter's file: in a zone with daylight saving, a day is the local calendar day,
    /// so the day the clocks spring forward is 23 hours long and still starts at midnight.
    #[test]
    fn date_utils_count_local_calendar_days_across_a_change_of_offset() {
        use std::rc::Rc;
        use waft_embedder::{PlatformRef, SignedDuration};
        use waft_test::{TestPlatform, TestTimeZone};
        /// 2026-03-29T01:00:00Z: clocks in Central Europe go from 2:00 to 3:00.
        const SPRING_FORWARD: i64 = 1_774_746_000;
        let central_europe = TestTimeZone::new(|seconds| {
            if seconds >= SPRING_FORWARD {
                (SignedDuration::from_hours(2), "CEST".to_string())
            } else {
                (SignedDuration::from_hours(1), "CET".to_string())
            }
        });
        let platform: PlatformRef = Rc::new(TestPlatform::new().with_time_zone(central_europe));
        let cell = AppCell::with_platform(platform);
        let app = cell.borrow();

        let late_evening = DateTime::new_with_time(&app, 2026, 3, 28, 23, 30, 0, 0, 0);
        let day = DateUtils::date_only(&app, &late_evening);
        assert_eq!((day.day(), day.hour()), (28, 0));
        let next = DateUtils::add_days_to_date(&app, &day, 1);
        let after = DateUtils::add_days_to_date(&app, &day, 2);
        assert_eq!((next.day(), next.hour()), (29, 0));
        assert_eq!((after.day(), after.hour()), (30, 0));
        assert_eq!(
            after.microseconds_since_epoch() - next.microseconds_since_epoch(),
            23 * 60 * 60 * 1_000_000,
            "the day the clocks go forward has 23 hours"
        );
        assert!(DateUtils::is_same_day(Some(&late_evening), Some(&day)));
    }

    #[test]
    fn date_utils_date_only_drops_the_time() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let date = DateTime::new_with_time(&app, 2019, 6, 15, 13, 45, 30, 0, 0);
        assert_eq!(
            DateUtils::date_only(&app, &date),
            DateTime::new(&app, 2019, 6, 15)
        );
    }

    #[test]
    fn date_utils_dates_only_drops_the_times_of_a_range() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let range = DateTimeRange::new(
            DateTime::new_with_time(&app, 2019, 6, 15, 13, 0, 0, 0, 0),
            DateTime::new_with_time(&app, 2019, 6, 20, 1, 0, 0, 0, 0),
        );
        let dates = DateUtils::dates_only(&app, &range);
        assert_eq!(dates.start, DateTime::new(&app, 2019, 6, 15));
        assert_eq!(dates.end, DateTime::new(&app, 2019, 6, 20));
    }

    #[test]
    fn date_utils_is_same_day_and_month_compare_the_calendar_fields() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let morning = DateTime::new_with_time(&app, 2019, 6, 15, 1, 0, 0, 0, 0);
        let evening = DateTime::new_with_time(&app, 2019, 6, 15, 23, 0, 0, 0, 0);
        let next_day = DateTime::new(&app, 2019, 6, 16);
        let next_year = DateTime::new(&app, 2020, 6, 15);
        assert!(DateUtils::is_same_day(Some(&morning), Some(&evening)));
        assert!(!DateUtils::is_same_day(Some(&morning), Some(&next_day)));
        assert!(!DateUtils::is_same_day(Some(&morning), Some(&next_year)));
        assert!(DateUtils::is_same_day(None, None));
        assert!(!DateUtils::is_same_day(Some(&morning), None));
        assert!(DateUtils::is_same_month(Some(&morning), Some(&next_day)));
        assert!(!DateUtils::is_same_month(Some(&morning), Some(&next_year)));
        assert!(DateUtils::is_same_month(None, None));
    }

    #[test]
    fn date_utils_month_delta_counts_whole_months() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let date1 = DateTime::new(&app, 2019, 6, 15);
        let date2 = DateTime::new(&app, 2020, 1, 15);
        assert_eq!(DateUtils::month_delta(&date1, &date2), 7);
        assert_eq!(DateUtils::month_delta(&date2, &date1), -7);
    }

    #[test]
    fn date_utils_adds_months_and_days_across_year_ends() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let december = DateTime::new(&app, 2019, 12, 1);
        assert_eq!(
            DateUtils::add_months_to_month_date(&app, &december, 1),
            DateTime::new(&app, 2020, 1, 1)
        );
        assert_eq!(
            DateUtils::add_months_to_month_date(&app, &DateTime::new(&app, 2020, 1, 1), -1),
            december
        );
        assert_eq!(
            DateUtils::add_days_to_date(&app, &DateTime::new(&app, 2019, 12, 31), 1),
            DateTime::new(&app, 2020, 1, 1)
        );
        assert_eq!(
            DateUtils::add_days_to_date(&app, &DateTime::new(&app, 2020, 3, 1), -1),
            DateTime::new(&app, 2020, 2, 29)
        );
    }

    #[test]
    fn date_utils_get_days_in_month_follows_the_gregorian_leap_rule() {
        assert_eq!(DateUtils::get_days_in_month(2019, 2), 28);
        assert_eq!(DateUtils::get_days_in_month(2020, 2), 29);
        assert_eq!(DateUtils::get_days_in_month(1900, 2), 28);
        assert_eq!(DateUtils::get_days_in_month(2000, 2), 29);
        assert_eq!(DateUtils::get_days_in_month(2019, 1), 31);
        assert_eq!(DateUtils::get_days_in_month(2019, 4), 30);
        assert_eq!(DateUtils::get_days_in_month(2019, 12), 31);
    }

    #[test]
    fn date_utils_first_day_offset_counts_the_blanks_before_the_first() {
        // September 1, 2017 is a Friday, and English weeks start on Sunday: five blanks.
        let english = DefaultMaterialLocalizations;
        assert_eq!(DateUtils::first_day_offset(2017, 9, &english), 5);
    }

    #[test]
    fn date_time_range_has_the_duration_between_its_dates() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let range = DateTimeRange::new(
            DateTime::new(&app, 2019, 6, 15),
            DateTime::new(&app, 2019, 6, 20),
        );
        assert_eq!(range.duration(), Duration::from_secs(5 * 24 * 60 * 60));
        assert_eq!(
            range.to_string(),
            "2019-06-15 00:00:00.000 - 2019-06-20 00:00:00.000"
        );
    }

    #[test]
    #[should_panic]
    fn date_time_range_asserts_that_start_is_not_after_end() {
        let cell = AppCell::new();
        let app = cell.borrow();
        DateTimeRange::new(
            DateTime::new(&app, 2019, 6, 20),
            DateTime::new(&app, 2019, 6, 15),
        );
    }

    #[test]
    fn the_gregorian_delegate_builds_months_and_days() {
        let cell = AppCell::new();
        let app = cell.borrow();
        let delegate = GregorianCalendarDelegate;
        assert_eq!(
            delegate.get_month(&app, 2019, 3),
            DateTime::new(&app, 2019, 3, 1)
        );
        assert_eq!(
            delegate.get_day(&app, 2019, 3, 4),
            DateTime::new(&app, 2019, 3, 4)
        );
        assert_eq!(delegate.get_days_in_month(2020, 2), 29);
    }
}
