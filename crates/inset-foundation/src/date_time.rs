//! dart:core `DateTime`, for the calendar arithmetic the framework does.
//!
//! Only UTC exists: Rust's standard library has no time zone database, so the local-time
//! constructors and `toLocal` wait for a host that supplies one.

use std::cmp::Ordering;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// An instant in time, such as July 20, 1969, 8:18pm GMT.
///
/// DateTimes can represent time values that are at a distance of at most
/// 100,000,000 days from epoch (1970-01-01 UTC): -271821-04-20 to 275760-09-13.
///
/// Create a `DateTime` object by using one of the constructors or by parsing a
/// correctly formatted string, which complies with a subset of ISO 8601.
///
/// ```
/// # use inset_foundation::DateTime;
/// let moon_landing = DateTime::utc_with_time(1969, 7, 20, 20, 18, 4, 0, 0);
/// assert_eq!(moon_landing.weekday(), DateTime::SUNDAY);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateTime {
    microseconds_since_epoch: i64,
    is_utc: bool,
}

const MICROSECONDS_PER_MILLISECOND: i64 = 1000;
const MICROSECONDS_PER_SECOND: i64 = 1_000_000;
const MICROSECONDS_PER_MINUTE: i64 = 60 * MICROSECONDS_PER_SECOND;
const MICROSECONDS_PER_HOUR: i64 = 60 * MICROSECONDS_PER_MINUTE;
const MICROSECONDS_PER_DAY: i64 = 24 * MICROSECONDS_PER_HOUR;

/// The distance from epoch a `DateTime` can represent, in milliseconds: 100,000,000 days.
const MAX_MILLISECONDS_SINCE_EPOCH: i64 = 8_640_000_000_000_000;

impl DateTime {
    pub const MONDAY: i32 = 1;
    pub const TUESDAY: i32 = 2;
    pub const WEDNESDAY: i32 = 3;
    pub const THURSDAY: i32 = 4;
    pub const FRIDAY: i32 = 5;
    pub const SATURDAY: i32 = 6;
    pub const SUNDAY: i32 = 7;
    pub const DAYS_PER_WEEK: i32 = 7;

    pub const JANUARY: i32 = 1;
    pub const FEBRUARY: i32 = 2;
    pub const MARCH: i32 = 3;
    pub const APRIL: i32 = 4;
    pub const MAY: i32 = 5;
    pub const JUNE: i32 = 6;
    pub const JULY: i32 = 7;
    pub const AUGUST: i32 = 8;
    pub const SEPTEMBER: i32 = 9;
    pub const OCTOBER: i32 = 10;
    pub const NOVEMBER: i32 = 11;
    pub const DECEMBER: i32 = 12;
    pub const MONTHS_PER_YEAR: i32 = 12;

    /// Constructs a [`DateTime`] instance specified in the UTC time zone at midnight.
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let moon_landing = DateTime::utc(1969, 7, 20);
    /// ```
    ///
    /// Dart's `DateTime.utc(year, [month, day])`; the out-of-range values a caller passes
    /// (a thirteenth month, a zeroth day) roll into the neighbouring month or year.
    pub fn utc(year: i32, month: i32, day: i32) -> DateTime {
        DateTime::utc_with_time(year, month, day, 0, 0, 0, 0, 0)
    }

    /// Constructs a [`DateTime`] instance specified in the UTC time zone.
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let moon_landing = DateTime::utc_with_time(1969, 7, 20, 20, 18, 4, 0, 0);
    /// ```
    ///
    /// Dart's `DateTime.utc(year, month, day, hour, minute, second, millisecond,
    /// microsecond)`; every field may overflow into the next larger one.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's positional constructor; each field may overflow into the next"
    )]
    pub fn utc_with_time(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: i32,
        millisecond: i32,
        microsecond: i32,
    ) -> DateTime {
        let days = days_from_civil(year, month, day);
        let microseconds = days * MICROSECONDS_PER_DAY
            + i64::from(hour) * MICROSECONDS_PER_HOUR
            + i64::from(minute) * MICROSECONDS_PER_MINUTE
            + i64::from(second) * MICROSECONDS_PER_SECOND
            + i64::from(millisecond) * MICROSECONDS_PER_MILLISECOND
            + i64::from(microsecond);
        DateTime::from_microseconds_since_epoch(microseconds)
    }

    /// Constructs a [`DateTime`] instance with current date and time in the UTC time zone.
    ///
    /// Dart's `DateTime.now()` is local time; only UTC exists here.
    pub fn now() -> DateTime {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is past 1970");
        let microseconds = i64::try_from(since_epoch.as_micros()).expect("within range");
        DateTime::from_microseconds_since_epoch(microseconds)
    }

    /// Constructs a new [`DateTime`] instance with the given `milliseconds_since_epoch`.
    ///
    /// The constructed [`DateTime`] represents `1970-01-01T00:00:00Z + milliseconds_since_epoch
    /// ms` in UTC (Dart's `isUtc: true`).
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let new_year_utc = DateTime::from_milliseconds_since_epoch(1_640_995_200_000);
    /// assert_eq!(new_year_utc.year(), 2022);
    /// ```
    pub fn from_milliseconds_since_epoch(milliseconds_since_epoch: i64) -> DateTime {
        DateTime::from_microseconds_since_epoch(
            milliseconds_since_epoch * MICROSECONDS_PER_MILLISECOND,
        )
    }

    /// Constructs a new [`DateTime`] instance with the given `microseconds_since_epoch`.
    ///
    /// The constructed [`DateTime`] represents `1970-01-01T00:00:00Z + microseconds_since_epoch
    /// us` in UTC (Dart's `isUtc: true`).
    pub fn from_microseconds_since_epoch(microseconds_since_epoch: i64) -> DateTime {
        assert!(
            microseconds_since_epoch.abs()
                <= MAX_MILLISECONDS_SINCE_EPOCH * MICROSECONDS_PER_MILLISECOND,
            "DateTime is outside valid range: {microseconds_since_epoch}"
        );
        DateTime {
            microseconds_since_epoch,
            is_utc: true,
        }
    }

    /// True if this [`DateTime`] is set to UTC time.
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let moon_landing = DateTime::utc(1969, 7, 20);
    /// assert!(moon_landing.is_utc());
    /// ```
    pub fn is_utc(&self) -> bool {
        self.is_utc
    }

    /// The number of milliseconds since the "Unix epoch" 1970-01-01T00:00:00Z (UTC).
    ///
    /// This value is independent of the time zone.
    ///
    /// This value is at most 8,640,000,000,000,000ms (100,000,000 days) from the Unix
    /// epoch. In other words: `milliseconds_since_epoch.abs() <= 8640000000000000`.
    pub fn milliseconds_since_epoch(&self) -> i64 {
        self.microseconds_since_epoch
            .div_euclid(MICROSECONDS_PER_MILLISECOND)
    }

    /// The number of microseconds since the "Unix epoch" 1970-01-01T00:00:00Z (UTC).
    ///
    /// This value is independent of the time zone.
    ///
    /// This value is at most 8,640,000,000,000,000,000us (100,000,000 days) from the Unix
    /// epoch. In other words: `microseconds_since_epoch.abs() <= 8640000000000000000`.
    pub fn microseconds_since_epoch(&self) -> i64 {
        self.microseconds_since_epoch
    }

    /// The year.
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let moon_landing = DateTime::utc(1969, 7, 20);
    /// assert_eq!(moon_landing.year(), 1969);
    /// ```
    pub fn year(&self) -> i32 {
        self.civil().0
    }

    /// The month `[1..12]`.
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let moon_landing = DateTime::utc(1969, 7, 20);
    /// assert_eq!(moon_landing.month(), 7);
    /// assert_eq!(moon_landing.month(), DateTime::JULY);
    /// ```
    pub fn month(&self) -> i32 {
        self.civil().1
    }

    /// The day of the month `[1..31]`.
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let moon_landing = DateTime::utc(1969, 7, 20);
    /// assert_eq!(moon_landing.day(), 20);
    /// ```
    pub fn day(&self) -> i32 {
        self.civil().2
    }

    /// The hour of the day, expressed as in a 24-hour clock `[0..23]`.
    pub fn hour(&self) -> i32 {
        self.time_of_day_part(MICROSECONDS_PER_HOUR, 24)
    }

    /// The minute `[0...59]`.
    pub fn minute(&self) -> i32 {
        self.time_of_day_part(MICROSECONDS_PER_MINUTE, 60)
    }

    /// The second `[0...59]`.
    pub fn second(&self) -> i32 {
        self.time_of_day_part(MICROSECONDS_PER_SECOND, 60)
    }

    /// The millisecond `[0...999]`.
    pub fn millisecond(&self) -> i32 {
        self.time_of_day_part(MICROSECONDS_PER_MILLISECOND, 1000)
    }

    /// The microsecond `[0...999]`.
    pub fn microsecond(&self) -> i32 {
        self.time_of_day_part(1, 1000)
    }

    /// The day of the week [`MONDAY`](Self::MONDAY)..[`SUNDAY`](Self::SUNDAY).
    ///
    /// In accordance with ISO 8601 a week starts with Monday, which has the value 1.
    ///
    /// ```
    /// # use inset_foundation::DateTime;
    /// let moon_landing = DateTime::utc(1969, 7, 20);
    /// assert_eq!(moon_landing.weekday(), 7);
    /// assert_eq!(moon_landing.weekday(), DateTime::SUNDAY);
    /// ```
    pub fn weekday(&self) -> i32 {
        // The epoch was a Thursday.
        let days = self.days_since_epoch();
        i32::try_from((days + 3).rem_euclid(7)).expect("a weekday") + 1
    }

    /// The time zone name.
    ///
    /// This value is provided by the operating system and may be an
    /// abbreviation or a full name.
    ///
    /// In the current implementation, the time zone name is "UTC" for UTC dates.
    pub fn time_zone_name(&self) -> &'static str {
        "UTC"
    }

    /// The time zone offset, which is the difference between local time and UTC.
    ///
    /// The offset is positive for time zones east of UTC.
    ///
    /// Note, that JavaScript, Python and C return the difference between UTC and local
    /// time. Java, C# and Ruby return the difference between local time and UTC.
    ///
    /// For example, using local time in San Francisco, United States:
    /// ```text
    /// final dateUS = DateTime.parse('2021-11-09 10:00:00.000-0800');
    /// print(dateUS.timeZoneOffset); // -8:00:00.000000
    /// ```
    pub fn time_zone_offset(&self) -> Duration {
        Duration::ZERO
    }

    /// Returns this DateTime value in the UTC time zone.
    ///
    /// Returns `this` if it is already in UTC.
    pub fn to_utc(&self) -> DateTime {
        *self
    }

    /// Returns a new [`DateTime`] instance with `duration` added to `self`.
    ///
    /// ```
    /// # use std::time::Duration;
    /// # use inset_foundation::DateTime;
    /// let today = DateTime::utc(2022, 1, 1);
    /// let fifty_days_from_now = today.add(Duration::from_secs(50 * 24 * 60 * 60));
    /// assert_eq!(fifty_days_from_now.month(), 2);
    /// ```
    ///
    /// Notice that the duration being added is actually 50 * 24 * 60 * 60
    /// seconds. If the resulting `DateTime` has a different daylight saving offset
    /// than `self`, then the result won't have the same time-of-day as `self`, and
    /// may not even hit the calendar date 50 days later.
    ///
    /// Be careful when working with dates in local time.
    pub fn add(&self, duration: Duration) -> DateTime {
        self.offset(duration_microseconds(duration))
    }

    /// Returns a new [`DateTime`] instance with `duration` subtracted from `self`.
    ///
    /// ```
    /// # use std::time::Duration;
    /// # use inset_foundation::DateTime;
    /// let today = DateTime::utc(2022, 1, 1);
    /// let fifty_days_ago = today.subtract(Duration::from_secs(50 * 24 * 60 * 60));
    /// assert_eq!(fifty_days_ago.year(), 2021);
    /// ```
    ///
    /// Notice that the duration being subtracted is actually 50 * 24 * 60 * 60
    /// seconds. If the resulting `DateTime` has a different daylight saving offset
    /// than `self`, then the result won't have the same time-of-day as `self`, and
    /// may not even hit the calendar date 50 days earlier.
    ///
    /// Be careful when working with dates in local time.
    pub fn subtract(&self, duration: Duration) -> DateTime {
        self.offset(-duration_microseconds(duration))
    }

    /// Returns true if `self` occurs before `other`.
    ///
    /// The comparison is independent of whether the time is in UTC or in the local time
    /// zone.
    pub fn is_before(&self, other: &DateTime) -> bool {
        self.microseconds_since_epoch < other.microseconds_since_epoch
    }

    /// Returns true if `self` occurs after `other`.
    ///
    /// The comparison is independent of whether the time is in UTC or in the local time
    /// zone.
    pub fn is_after(&self, other: &DateTime) -> bool {
        self.microseconds_since_epoch > other.microseconds_since_epoch
    }

    /// Returns true if `self` occurs at the same moment as `other`.
    ///
    /// The comparison is independent of whether the time is in UTC or in the local time
    /// zone.
    pub fn is_at_same_moment_as(&self, other: &DateTime) -> bool {
        self.microseconds_since_epoch == other.microseconds_since_epoch
    }

    /// Compares this DateTime object to `other`, returning zero if the values are equal.
    ///
    /// A [`compare_to`](Self::compare_to) function returns:
    ///  * a negative value if this DateTime [`is_before`](Self::is_before) `other`.
    ///  * `0` if this DateTime [`is_at_same_moment_as`](Self::is_at_same_moment_as) `other`,
    ///    and
    ///  * a positive value otherwise (when this DateTime [`is_after`](Self::is_after)
    ///    `other`).
    pub fn compare_to(&self, other: &DateTime) -> Ordering {
        self.microseconds_since_epoch
            .cmp(&other.microseconds_since_epoch)
    }

    /// Returns an ISO-8601 full-precision extended format representation.
    ///
    /// The format is `yyyy-MM-ddTHH:mm:ss.mmmuuuZ` for UTC time, and
    /// `yyyy-MM-ddTHH:mm:ss.mmmuuu` (no trailing "Z") for local/non-UTC time,
    /// where:
    ///
    /// * `yyyy` is a, possibly negative, four digit representation of the year,
    ///   if the year is in the range -9999 to 9999,
    ///   otherwise it is a signed six digit representation of the year.
    /// * `MM` is the month in the range 01 to 12,
    /// * `dd` is the day of the month in the range 01 to 31,
    /// * `HH` are hours in the range 00 to 23,
    /// * `mm` are minutes in the range 00 to 59,
    /// * `ss` are seconds in the range 00 to 59 (no leap seconds),
    /// * `mmm` are milliseconds in the range 000 to 999, and
    /// * `uuu` are microseconds in the range 001 to 999. If
    ///   [`microsecond`](Self::microsecond) equals 0, then this part is omitted.
    pub fn to_iso8601_string(&self) -> String {
        self.format('T')
    }

    fn format(&self, separator: char) -> String {
        let year = self.year();
        let y = if (-9999..=9999).contains(&year) {
            four_digits(year)
        } else {
            six_digits(year)
        };
        let us = match self.microsecond() {
            0 => String::new(),
            microsecond => three_digits(microsecond),
        };
        let z = if self.is_utc { "Z" } else { "" };
        format!(
            "{y}-{:02}-{:02}{separator}{:02}:{:02}:{:02}.{:03}{us}{z}",
            self.month(),
            self.day(),
            self.hour(),
            self.minute(),
            self.second(),
            self.millisecond()
        )
    }

    fn offset(&self, microseconds: i64) -> DateTime {
        DateTime::from_microseconds_since_epoch(self.microseconds_since_epoch + microseconds)
    }

    fn days_since_epoch(&self) -> i64 {
        self.microseconds_since_epoch
            .div_euclid(MICROSECONDS_PER_DAY)
    }

    fn civil(&self) -> (i32, i32, i32) {
        civil_from_days(self.days_since_epoch())
    }

    fn time_of_day_part(&self, unit: i64, modulus: i64) -> i32 {
        let part = self
            .microseconds_since_epoch
            .div_euclid(unit)
            .rem_euclid(modulus);
        i32::try_from(part).expect("below the modulus")
    }
}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &DateTime) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTime {
    fn cmp(&self, other: &DateTime) -> Ordering {
        self.compare_to(other)
    }
}

/// Returns a human-readable string for this instance.
///
/// The returned string is constructed for the time zone of this instance.
/// The `toString` method on:
///
/// * UTC dates ends with 'Z'.
/// * Local dates have no suffix.
///
/// For example, `yyyy-MM-dd HH:mm:ss.mmmuuuZ` for UTC time.
impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format(' '))
    }
}

fn duration_microseconds(duration: Duration) -> i64 {
    i64::try_from(duration.as_micros()).expect("a duration within the DateTime range")
}

fn four_digits(n: i32) -> String {
    let sign = if n < 0 { "-" } else { "" };
    format!("{sign}{:04}", n.abs())
}

fn six_digits(n: i32) -> String {
    debug_assert!(!(-9999..=9999).contains(&n));
    let sign = if n < 0 { "-" } else { "+" };
    format!("{sign}{:06}", n.abs())
}

fn three_digits(n: i32) -> String {
    format!("{n:03}")
}

/// Days since the epoch of the given proleptic Gregorian date; the month rolls into the
/// year and the day into the month, as Dart's constructors let them.
fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    let year = i64::from(year) + i64::from(month - 1).div_euclid(12);
    let month = i64::from(month - 1).rem_euclid(12) + 1;
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468 + i64::from(day - 1)
}

/// The proleptic Gregorian (year, month, day) of a day count since the epoch.
fn civil_from_days(days: i64) -> (i32, i32, i32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (
        i32::try_from(year).expect("a year in range"),
        i32::try_from(month).expect("a month"),
        i32::try_from(day).expect("a day"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_a_thursday_and_the_fields_read_back() {
        let epoch = DateTime::from_microseconds_since_epoch(0);
        assert_eq!(
            (epoch.year(), epoch.month(), epoch.day()),
            (1970, DateTime::JANUARY, 1)
        );
        assert_eq!(epoch.weekday(), DateTime::THURSDAY);
        let moon = DateTime::utc_with_time(1969, 7, 20, 20, 18, 4, 5, 6);
        assert_eq!((moon.hour(), moon.minute(), moon.second()), (20, 18, 4));
        assert_eq!((moon.millisecond(), moon.microsecond()), (5, 6));
        assert_eq!(moon.weekday(), DateTime::SUNDAY);
        assert!(moon.is_before(&epoch));
        assert!(epoch.is_after(&moon));
        assert_eq!(moon.compare_to(&epoch), Ordering::Less);
    }

    #[test]
    fn leap_years_and_out_of_range_fields_roll_over_like_dart() {
        assert_eq!(DateTime::utc(2024, 2, 29).day(), 29);
        assert_eq!(DateTime::utc(2024, 2, 30).month(), DateTime::MARCH);
        assert_eq!(DateTime::utc(1900, 2, 29).month(), DateTime::MARCH);
        assert_eq!(DateTime::utc(2000, 2, 29).month(), DateTime::FEBRUARY);
        let thirteenth_month = DateTime::utc(2025, 13, 1);
        assert_eq!(
            (thirteenth_month.year(), thirteenth_month.month()),
            (2026, DateTime::JANUARY)
        );
        let zeroth_day = DateTime::utc(2026, 3, 0);
        assert_eq!(
            (zeroth_day.month(), zeroth_day.day()),
            (DateTime::FEBRUARY, 28)
        );
        let negative_month = DateTime::utc(2026, -1, 1);
        assert_eq!(
            (negative_month.year(), negative_month.month()),
            (2025, DateTime::NOVEMBER)
        );
        assert_eq!(DateTime::utc_with_time(2026, 1, 1, 25, 0, 0, 0, 0).day(), 2);
    }

    #[test]
    fn add_and_subtract_move_across_month_boundaries() {
        let fifty_days = Duration::from_secs(50 * 24 * 60 * 60);
        let today = DateTime::utc(2022, 1, 1);
        assert_eq!(
            today.add(fifty_days).to_string(),
            "2022-02-20 00:00:00.000Z"
        );
        assert_eq!(
            today.subtract(fifty_days).to_string(),
            "2021-11-12 00:00:00.000Z"
        );
        assert_eq!(today.milliseconds_since_epoch(), 1_640_995_200_000);
        assert_eq!(
            DateTime::from_milliseconds_since_epoch(1_640_995_200_000),
            today
        );
    }

    #[test]
    fn formatting_follows_dart() {
        let moon = DateTime::utc_with_time(1969, 7, 20, 20, 18, 4, 0, 0);
        assert_eq!(moon.to_string(), "1969-07-20 20:18:04.000Z");
        assert_eq!(moon.to_iso8601_string(), "1969-07-20T20:18:04.000Z");
        let precise = DateTime::utc_with_time(2026, 9, 5, 1, 2, 3, 4, 5);
        assert_eq!(precise.to_iso8601_string(), "2026-09-05T01:02:03.004005Z");
        assert_eq!(
            DateTime::utc(-44, 3, 15).to_string(),
            "-0044-03-15 00:00:00.000Z"
        );
        assert_eq!(
            DateTime::utc(12345, 1, 1).to_iso8601_string(),
            "+012345-01-01T00:00:00.000Z"
        );
    }

    #[test]
    fn dates_before_the_epoch_read_back_their_fields() {
        let date = DateTime::utc_with_time(1960, 12, 31, 23, 59, 59, 999, 999);
        assert_eq!((date.year(), date.month(), date.day()), (1960, 12, 31));
        assert_eq!((date.hour(), date.minute(), date.second()), (23, 59, 59));
        assert_eq!((date.millisecond(), date.microsecond()), (999, 999));
        assert_eq!(
            date.add(Duration::from_micros(1)),
            DateTime::utc(1961, 1, 1)
        );
    }
}
