//! dart:core `DateTime`, in UTC or in the local time zone.
//!
//! Local time comes from the system on every call that needs it (`local_time`), as the Dart VM's
//! natives ask it: a local value works out its offset once, when it is made, for its fields, as
//! Dart's `__parts` are worked out once, and `time_zone_offset` / `time_zone_name` ask again.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use web_time::{SystemTime, UNIX_EPOCH};

use crate::local_time;
use crate::signed_duration::SignedDuration;

/// An instant in time, such as July 20, 1969, 8:18pm GMT.
///
/// DateTimes can represent time values that are at a distance of at most
/// 100,000,000 days from epoch (1970-01-01 UTC): -271821-04-20 to 275760-09-13.
///
/// Create a `DateTime` object by using one of the constructors or by parsing a
/// correctly formatted string, which complies with a subset of ISO 8601.
///
/// ```
/// # use waft_foundation::DateTime;
/// let moon_landing = DateTime::utc_with_time(1969, 7, 20, 20, 18, 4, 0, 0);
/// assert_eq!(moon_landing.weekday(), DateTime::SUNDAY);
/// ```
///
/// A value is UTC or local. Dart's `==` compares the instant and whether it is UTC, so a local
/// value and the UTC value of the same instant are not equal; [`is_at_same_moment_as`] compares
/// the instant alone.
///
/// [`is_at_same_moment_as`]: Self::is_at_same_moment_as
#[derive(Clone, Copy, Debug)]
pub struct DateTime {
    microseconds_since_epoch: i64,
    is_utc: bool,
    /// The local time's offset from UTC at this instant, in microseconds, asked of the system
    /// once when the value is made, as Dart's fields are worked out once: zero for UTC.
    local_offset: i64,
}

const MICROSECONDS_PER_MILLISECOND: i64 = 1000;
const MICROSECONDS_PER_SECOND: i64 = 1_000_000;
const MICROSECONDS_PER_MINUTE: i64 = 60 * MICROSECONDS_PER_SECOND;
const MICROSECONDS_PER_HOUR: i64 = 60 * MICROSECONDS_PER_MINUTE;
const MICROSECONDS_PER_DAY: i64 = 24 * MICROSECONDS_PER_HOUR;
const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

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

    /// Constructs a [`DateTime`] instance specified in the local time zone at midnight.
    ///
    /// Dart's `DateTime(year, [month, day])`; the out-of-range values a caller passes (a
    /// thirteenth month, a zeroth day) roll into the neighbouring month or year.
    ///
    /// ```
    /// # use waft_foundation::DateTime;
    /// let annual_meeting = DateTime::new(2023, 9, 25);
    /// assert!(!annual_meeting.is_utc());
    /// assert_eq!((annual_meeting.month(), annual_meeting.hour()), (9, 0));
    /// ```
    pub fn new(year: i32, month: i32, day: i32) -> DateTime {
        DateTime::new_with_time(year, month, day, 0, 0, 0, 0, 0)
    }

    /// Constructs a [`DateTime`] instance specified in the local time zone.
    ///
    /// Dart's `DateTime(year, month, day, hour, minute, second, millisecond, microsecond)`;
    /// every field may overflow into the next larger one. A wall time a daylight-saving change
    /// skips is taken in the zone before the change, so 2:30 on the morning clocks go from 2:00
    /// to 3:00 is 3:30; one the change repeats is the earlier of the two.
    ///
    /// # Panics
    ///
    /// If the time is outside the range a [`DateTime`] holds.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's positional constructor; each field may overflow into the next"
    )]
    pub fn new_with_time(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: i32,
        millisecond: i32,
        microsecond: i32,
    ) -> DateTime {
        let fields = [
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            microsecond,
        ];
        DateTime::from_fields(fields, false)
    }

    /// Constructs a [`DateTime`] instance specified in the UTC time zone at midnight.
    ///
    /// ```
    /// # use waft_foundation::DateTime;
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
    /// # use waft_foundation::DateTime;
    /// let moon_landing = DateTime::utc_with_time(1969, 7, 20, 20, 18, 4, 0, 0);
    /// ```
    ///
    /// Dart's `DateTime.utc(year, month, day, hour, minute, second, millisecond,
    /// microsecond)`; every field may overflow into the next larger one.
    ///
    /// # Panics
    ///
    /// If the time is outside the range a [`DateTime`] holds.
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
        let fields = [
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            microsecond,
        ];
        DateTime::from_fields(fields, true)
    }

    /// Constructs a [`DateTime`] instance with current date and time in the local time zone.
    ///
    /// ```
    /// # use waft_foundation::DateTime;
    /// let now = DateTime::now();
    /// assert!(!now.is_utc());
    /// ```
    pub fn now() -> DateTime {
        DateTime::from_value(current_microseconds(), false)
    }

    /// Constructs a [`DateTime`] with the current UTC date and time: Dart's
    /// `DateTime.timestamp()`.
    ///
    /// ```
    /// # use waft_foundation::DateTime;
    /// let now = DateTime::timestamp();
    /// assert!(now.is_utc());
    /// ```
    pub fn timestamp() -> DateTime {
        DateTime::from_value(current_microseconds(), true)
    }

    /// Constructs a new [`DateTime`] instance with the given `milliseconds_since_epoch`.
    ///
    /// If `is_utc` is false then the date is in the local time zone. Dart's `isUtc` defaults to
    /// false.
    ///
    /// The constructed [`DateTime`] represents `1970-01-01T00:00:00Z + milliseconds_since_epoch
    /// ms` in the given time zone (local or UTC).
    ///
    /// ```
    /// # use waft_foundation::DateTime;
    /// let new_year_utc = DateTime::from_milliseconds_since_epoch(1_640_995_200_000, true);
    /// assert_eq!(new_year_utc.year(), 2022);
    /// ```
    ///
    /// # Panics
    ///
    /// If `milliseconds_since_epoch` is outside the range a [`DateTime`] holds.
    pub fn from_milliseconds_since_epoch(milliseconds_since_epoch: i64, is_utc: bool) -> DateTime {
        assert!(
            milliseconds_since_epoch.abs() <= MAX_MILLISECONDS_SINCE_EPOCH,
            "DateTime is outside valid range: {milliseconds_since_epoch}"
        );
        DateTime::from_value(
            milliseconds_since_epoch * MICROSECONDS_PER_MILLISECOND,
            is_utc,
        )
    }

    /// Constructs a new [`DateTime`] instance with the given `microseconds_since_epoch`.
    ///
    /// If `is_utc` is false then the date is in the local time zone. Dart's `isUtc` defaults to
    /// false.
    ///
    /// The constructed [`DateTime`] represents `1970-01-01T00:00:00Z + microseconds_since_epoch
    /// us` in the given time zone (local or UTC).
    ///
    /// # Panics
    ///
    /// If `microseconds_since_epoch` is outside the range a [`DateTime`] holds.
    pub fn from_microseconds_since_epoch(microseconds_since_epoch: i64, is_utc: bool) -> DateTime {
        DateTime::from_value(microseconds_since_epoch, is_utc)
    }

    /// The value at `microseconds_since_epoch`, with its local offset worked out for its fields.
    fn from_value(microseconds_since_epoch: i64, is_utc: bool) -> DateTime {
        assert!(
            microseconds_since_epoch.abs()
                <= MAX_MILLISECONDS_SINCE_EPOCH * MICROSECONDS_PER_MILLISECOND,
            "DateTime is outside valid range: {microseconds_since_epoch}"
        );
        let local_offset = if is_utc {
            0
        } else {
            local_offset_microseconds(microseconds_since_epoch)
        };
        DateTime {
            microseconds_since_epoch,
            is_utc,
            local_offset,
        }
    }

    /// Dart's `_internal`: the value whose fields are `fields`, in UTC or in the local zone.
    fn from_fields(fields: [i32; 8], is_utc: bool) -> DateTime {
        let value = broken_down_date_to_value(fields, is_utc)
            .unwrap_or_else(|| panic!("DateTime is outside valid range: {fields:?}"));
        DateTime::from_value(value, is_utc)
    }

    /// True if this [`DateTime`] is set to UTC time.
    ///
    /// ```
    /// # use waft_foundation::DateTime;
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
    /// # use waft_foundation::DateTime;
    /// let moon_landing = DateTime::utc(1969, 7, 20);
    /// assert_eq!(moon_landing.year(), 1969);
    /// ```
    pub fn year(&self) -> i32 {
        self.civil().0
    }

    /// The month `[1..12]`.
    ///
    /// ```
    /// # use waft_foundation::DateTime;
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
    /// # use waft_foundation::DateTime;
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
        // From the instant, as Dart's: every zone's offset is a whole number of seconds.
        let part = self
            .microseconds_since_epoch
            .div_euclid(MICROSECONDS_PER_MILLISECOND)
            .rem_euclid(1000);
        i32::try_from(part).expect("below a thousand")
    }

    /// The microsecond `[0...999]`.
    pub fn microsecond(&self) -> i32 {
        let part = self
            .microseconds_since_epoch
            .rem_euclid(MICROSECONDS_PER_MILLISECOND);
        i32::try_from(part).expect("below a thousand")
    }

    /// The day of the week [`MONDAY`](Self::MONDAY)..[`SUNDAY`](Self::SUNDAY).
    ///
    /// In accordance with ISO 8601 a week starts with Monday, which has the value 1.
    ///
    /// ```
    /// # use waft_foundation::DateTime;
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
    /// In the browser or on Unix-like systems commonly returns abbreviations,
    /// such as "CET" or "CEST". On Windows returns the full name, for example
    /// "Pacific Standard Time". Inside a WASI component it is the zone's IANA
    /// identifier, such as "Europe/Paris", or the host's description of a zone
    /// that has none, since WASI gives no abbreviation.
    ///
    /// The name is "UTC" for UTC dates.
    pub fn time_zone_name(&self) -> String {
        if self.is_utc {
            return "UTC".to_string();
        }
        local_time::zone_name(equivalent_seconds(self.microseconds_since_epoch))
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
    ///
    /// The system is asked again on every call, as Dart's is, so a value made before the zone
    /// changed answers the new zone's offset while its fields keep the old one's.
    pub fn time_zone_offset(&self) -> SignedDuration {
        if self.is_utc {
            return SignedDuration::ZERO;
        }
        SignedDuration::from_microseconds(local_offset_microseconds(self.microseconds_since_epoch))
    }

    /// Returns this DateTime value in the local time zone.
    ///
    /// Returns `self` if it is already in the local time zone. Otherwise this method is
    /// equivalent to:
    ///
    /// ```
    /// # use waft_foundation::DateTime;
    /// # let date = DateTime::utc(2026, 1, 1);
    /// let local = DateTime::from_microseconds_since_epoch(date.microseconds_since_epoch(), false);
    /// assert_eq!(local, date.to_local());
    /// ```
    pub fn to_local(&self) -> DateTime {
        if !self.is_utc {
            return *self;
        }
        DateTime::from_value(self.microseconds_since_epoch, false)
    }

    /// Returns this DateTime value in the UTC time zone.
    ///
    /// Returns `self` if it is already in UTC. Otherwise this method is equivalent to:
    ///
    /// ```
    /// # use waft_foundation::DateTime;
    /// # let date = DateTime::new(2026, 1, 1);
    /// let utc = DateTime::from_microseconds_since_epoch(date.microseconds_since_epoch(), true);
    /// assert_eq!(utc, date.to_utc());
    /// ```
    pub fn to_utc(&self) -> DateTime {
        if self.is_utc {
            return *self;
        }
        DateTime::from_value(self.microseconds_since_epoch, true)
    }

    /// Returns a new [`DateTime`] instance with `duration` added to `self`.
    ///
    /// ```
    /// # use std::time::Duration;
    /// # use waft_foundation::DateTime;
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
    /// # use waft_foundation::DateTime;
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
        DateTime::from_value(self.microseconds_since_epoch + microseconds, self.is_utc)
    }

    /// Dart's `_localDateInUtcMicros`: the instant whose UTC fields are this value's fields.
    fn local_microseconds(&self) -> i64 {
        self.microseconds_since_epoch + self.local_offset
    }

    fn days_since_epoch(&self) -> i64 {
        self.local_microseconds().div_euclid(MICROSECONDS_PER_DAY)
    }

    fn civil(&self) -> (i32, i32, i32) {
        civil_from_days(self.days_since_epoch())
    }

    fn time_of_day_part(&self, unit: i64, modulus: i64) -> i32 {
        let part = self
            .local_microseconds()
            .div_euclid(unit)
            .rem_euclid(modulus);
        i32::try_from(part).expect("below the modulus")
    }
}

/// Dart's `==`: the same instant, and both UTC or both local.
impl PartialEq for DateTime {
    fn eq(&self, other: &DateTime) -> bool {
        self.microseconds_since_epoch == other.microseconds_since_epoch
            && self.is_utc == other.is_utc
    }
}

impl Eq for DateTime {}

impl Hash for DateTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.microseconds_since_epoch.hash(state);
        self.is_utc.hash(state);
    }
}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &DateTime) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// By instant, as [`compare_to`](DateTime::compare_to), and then local before UTC: Rust's order
/// must agree with `==`, which tells a local value from the UTC value of the same instant, where
/// Dart's `compareTo` answers zero for the two.
impl Ord for DateTime {
    fn cmp(&self, other: &DateTime) -> Ordering {
        self.compare_to(other).then(self.is_utc.cmp(&other.is_utc))
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

/// The system clock now, in microseconds since the epoch: Dart's `_getCurrentMicros`.
fn current_microseconds() -> i64 {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is past 1970");
    i64::try_from(since_epoch.as_micros()).expect("within range")
}

/// The local zone's offset at the instant, in microseconds: Dart's `_timeZoneOffsetInSeconds`.
fn local_offset_microseconds(microseconds_since_epoch: i64) -> i64 {
    local_time::offset_in_seconds(equivalent_seconds(microseconds_since_epoch))
        * MICROSECONDS_PER_SECOND
}

/// Dart's `_brokenDownDateToValue`: the instant whose fields are `fields`, `None` if it is out
/// of range.
fn broken_down_date_to_value(fields: [i32; 8], is_utc: bool) -> Option<i64> {
    let [
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond,
        microsecond,
    ] = fields;
    // First compute the instant in UTC, independent of `is_utc`; the zone's offset comes after.
    let mut microseconds_since_epoch = days_from_civil(year, month, day) * MICROSECONDS_PER_DAY
        + i64::from(hour) * MICROSECONDS_PER_HOUR
        + i64::from(minute) * MICROSECONDS_PER_MINUTE
        + i64::from(second) * MICROSECONDS_PER_SECOND
        + i64::from(millisecond) * MICROSECONDS_PER_MILLISECOND
        + i64::from(microsecond);
    let max = MAX_MILLISECONDS_SINCE_EPOCH * MICROSECONDS_PER_MILLISECOND;
    if !is_utc {
        // The zone's offset is always less than a day, so a day of margin weeds out the values
        // no offset can bring into range before the system is asked.
        if microseconds_since_epoch.abs() > max + MICROSECONDS_PER_DAY {
            return None;
        }
        microseconds_since_epoch -= to_local_time_offset(microseconds_since_epoch);
    }
    (microseconds_since_epoch.abs() <= max).then_some(microseconds_since_epoch)
}

/// Dart's `_toLocalTimeOffset`: the offset, in microseconds, of the local time zone at the
/// instant whose local wall time is the UTC wall time of `microseconds_since_epoch`.
///
/// Where a daylight-saving change repeats that wall time, the earlier instant; where it skips
/// it, the instant that would have had that wall time in the zone before the change.
fn to_local_time_offset(microseconds_since_epoch: i64) -> i64 {
    // Start with the zone at the instant itself, which is within a day of the one looked for.
    let mut offset = local_offset_microseconds(microseconds_since_epoch);
    // An offset of zero is a solution already.
    if offset != 0 {
        let offset2 = local_offset_microseconds(microseconds_since_epoch - offset);
        if offset2 != offset {
            // A second zone within the same day; assume there are no more. Either the third
            // probe confirms the second, or there is no solution, and the lower offset (the
            // later time) wins.
            let offset3 = local_offset_microseconds(microseconds_since_epoch - offset2);
            return offset2.min(offset3);
        }
        offset = offset2;
    }
    // Look for an earlier zone that also has a solution, assuming a change of at most two
    // hours (double daylight saving happened, in part of Canada in 1988).
    let offset4 =
        local_offset_microseconds(microseconds_since_epoch - offset - 2 * MICROSECONDS_PER_HOUR);
    if offset4 > offset {
        if offset4 == offset + 2 * MICROSECONDS_PER_HOUR {
            return offset4;
        }
        let offset5 = local_offset_microseconds(microseconds_since_epoch - offset4);
        if offset5 == offset4 {
            return offset4;
        }
    }
    offset
}

/// Dart's `_equivalentSeconds`: the instant, in whole seconds, mapped into the range a 32-bit
/// `time_t` holds when it lies outside, by moving it to a year with the same leap-ness and the
/// same weekday on January 1st, so the system answers its rules for it.
fn equivalent_seconds(microseconds_since_epoch: i64) -> i64 {
    const CUT_OFF_SECONDS: i64 = 0x7FFF_FFFF;
    let mut seconds = microseconds_since_epoch.div_euclid(MICROSECONDS_PER_SECOND);
    if seconds.abs() > CUT_OFF_SECONDS {
        let (year, _, _) = civil_from_days(seconds.div_euclid(SECONDS_PER_DAY));
        let days = day_from_year(year);
        let equivalent_days = day_from_year(equivalent_year(year));
        seconds += (equivalent_days - days) * SECONDS_PER_DAY;
    }
    seconds
}

/// Days since the epoch of January 1st of `year`.
fn day_from_year(year: i32) -> i64 {
    days_from_civil(year, 1, 1)
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 16 == 0 || year % 100 != 0)
}

/// Dart's `_equivalentYear`, from V8: a year in 2008..2035 with the same leap-ness as `year`
/// and the same weekday on January 1st.
fn equivalent_year(year: i32) -> i32 {
    // January 1st's weekday, 0 for Sunday; 1970-01-01 was a Thursday.
    let week_day = (day_from_year(year) + 4).rem_euclid(7);
    let week_day = i32::try_from(week_day).expect("a weekday");
    // 1956-01-01 and 1967-01-01 were Sundays, and 1956 was a leap year. Twelve years move
    // January 1st's weekday on by one, so `week_day * 12` years later starts on the wanted day.
    let recent_year = if is_leap_year(year) { 1956 } else { 1967 } + week_day * 12;
    // Near 2008 the calendar repeats every 28 years.
    2008 + (recent_year - 2008).rem_euclid(28)
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
        let epoch = DateTime::from_microseconds_since_epoch(0, true);
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
            DateTime::from_milliseconds_since_epoch(1_640_995_200_000, true),
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

    use crate::local_time::tests::with_zone;

    const HOUR: i64 = 60 * 60;

    /// 2026-03-29T01:00:00Z: clocks in Central Europe go from 2:00 to 3:00.
    const SPRING_FORWARD: i64 = 1_774_746_000;
    /// 2026-10-25T01:00:00Z: clocks in Central Europe go from 3:00 back to 2:00.
    const FALL_BACK: i64 = 1_792_890_000;

    /// Central European time for 2026: UTC+1, and UTC+2 between the two changes.
    fn central_europe(seconds: i64) -> (i64, &'static str) {
        if (SPRING_FORWARD..FALL_BACK).contains(&seconds) {
            (2 * HOUR, "CEST")
        } else {
            (HOUR, "CET")
        }
    }

    /// Pacific standard time, all year: west of UTC, so the offset is negative.
    fn pacific(_seconds: i64) -> (i64, &'static str) {
        (-8 * HOUR, "PST")
    }

    #[test]
    fn a_local_value_reads_its_fields_in_the_zone_and_keeps_the_instant() {
        with_zone(central_europe, || {
            let epoch = DateTime::from_microseconds_since_epoch(0, false);
            assert!(!epoch.is_utc());
            assert_eq!((epoch.year(), epoch.month(), epoch.day()), (1970, 1, 1));
            assert_eq!(epoch.hour(), 1);
            assert_eq!(epoch.to_string(), "1970-01-01 01:00:00.000");
            assert_eq!(epoch.to_utc().to_string(), "1970-01-01 00:00:00.000Z");
            assert_eq!(epoch.time_zone_name(), "CET");
            assert_eq!(epoch.time_zone_offset(), SignedDuration::from_hours(1));
            assert_eq!(epoch.to_utc().time_zone_offset(), SignedDuration::ZERO);
            assert_eq!(epoch.to_utc().time_zone_name(), "UTC");
        });
    }

    #[test]
    fn the_offset_west_of_utc_is_negative() {
        with_zone(pacific, || {
            let date = DateTime::new_with_time(2021, 11, 9, 10, 0, 0, 0, 0);
            assert_eq!(date.time_zone_offset().to_string(), "-8:00:00.000000");
            assert_eq!(date.to_utc().hour(), 18);
            assert_eq!(date.milliseconds_since_epoch(), 1_636_480_800_000);
        });
    }

    /// `core/date_time_test.dart`'s round trips: the local constructor gives back its fields,
    /// and `toLocal` / `toUtc` keep the instant while `==` tells them apart.
    #[test]
    fn local_and_utc_values_of_one_instant_are_at_the_same_moment_but_not_equal() {
        with_zone(central_europe, || {
            let local = DateTime::new_with_time(2026, 7, 1, 12, 30, 15, 250, 7);
            assert_eq!(
                (
                    local.year(),
                    local.month(),
                    local.day(),
                    local.hour(),
                    local.minute()
                ),
                (2026, 7, 1, 12, 30)
            );
            assert_eq!(
                (local.second(), local.millisecond(), local.microsecond()),
                (15, 250, 7)
            );
            let utc = local.to_utc();
            assert_eq!(utc.hour(), 10);
            assert!(utc.is_at_same_moment_as(&local));
            assert_ne!(utc, local);
            assert_eq!(utc.to_local(), local);
            assert_eq!(local.to_local(), local);
        });
    }

    /// Dart's `_toLocalTimeOffset`: a wall time the change to summer time skips is taken in
    /// the zone before the change.
    #[test]
    fn a_skipped_wall_time_is_read_in_the_zone_before_the_change() {
        with_zone(central_europe, || {
            let skipped = DateTime::new_with_time(2026, 3, 29, 2, 30, 0, 0, 0);
            assert_eq!((skipped.hour(), skipped.minute()), (3, 30));
            assert_eq!(skipped.time_zone_name(), "CEST");
            assert_eq!(skipped.to_utc().to_string(), "2026-03-29 01:30:00.000Z");
        });
    }

    /// Dart's `_toLocalTimeOffset`: a wall time the change back to winter time repeats is the
    /// earlier of the two instants.
    #[test]
    fn a_repeated_wall_time_is_the_earlier_instant() {
        with_zone(central_europe, || {
            let repeated = DateTime::new_with_time(2026, 10, 25, 2, 30, 0, 0, 0);
            assert_eq!(repeated.to_utc().to_string(), "2026-10-25 00:30:00.000Z");
            assert_eq!(repeated.time_zone_name(), "CEST");
            let an_hour_later = repeated.add(Duration::from_secs(60 * 60));
            assert_eq!((an_hour_later.hour(), an_hour_later.minute()), (2, 30));
            assert_eq!(an_hour_later.time_zone_name(), "CET");
        });
    }

    /// Dart's `_equivalentSeconds`: outside the range of a 32-bit `time_t`, the system is asked
    /// about a year with the same calendar, so far dates read the zone's rules too.
    #[test]
    fn a_far_date_asks_the_zone_about_an_equivalent_year() {
        fn checked(seconds: i64) -> (i64, &'static str) {
            assert!(seconds.abs() <= 0x7FFF_FFFF, "asked about {seconds}");
            central_europe_any_year(seconds)
        }
        with_zone(checked, || {
            let summer = DateTime::new_with_time(2500, 7, 1, 12, 0, 0, 0, 0);
            assert_eq!(
                (summer.year(), summer.month(), summer.hour()),
                (2500, 7, 12)
            );
            assert_eq!(summer.time_zone_offset(), SignedDuration::from_hours(2));
            let winter = DateTime::new_with_time(1800, 1, 1, 12, 0, 0, 0, 0);
            assert_eq!(winter.time_zone_offset(), SignedDuration::from_hours(1));
            assert_eq!(winter.to_utc().hour(), 11);
        });
    }

    /// Summer time from April to September of whatever year the instant is in.
    fn central_europe_any_year(seconds: i64) -> (i64, &'static str) {
        let (_, month, _) = civil_from_days(seconds.div_euclid(SECONDS_PER_DAY));
        if (4..=9).contains(&month) {
            (2 * HOUR, "CEST")
        } else {
            (HOUR, "CET")
        }
    }

    #[test]
    fn equivalent_years_keep_the_calendar() {
        for year in [1600, 1800, 1900, 2100, 2400, 2500, 9999] {
            let equivalent = equivalent_year(year);
            assert!(
                (2008..=2035).contains(&equivalent),
                "{year} -> {equivalent}"
            );
            assert_eq!(is_leap_year(year), is_leap_year(equivalent), "{year}");
            assert_eq!(
                DateTime::utc(year, 1, 1).weekday(),
                DateTime::utc(equivalent, 1, 1).weekday(),
                "{year}"
            );
        }
    }

    #[test]
    fn now_is_local_and_timestamp_is_utc() {
        with_zone(pacific, || {
            let now = DateTime::now();
            let timestamp = DateTime::timestamp();
            assert!(!now.is_utc());
            assert!(timestamp.is_utc());
            assert!(
                (timestamp.microseconds_since_epoch() - now.microseconds_since_epoch()).abs()
                    < 60_000_000
            );
            assert_eq!(now.time_zone_name(), "PST");
        });
    }

    #[test]
    fn the_order_agrees_with_equality_at_one_instant() {
        with_zone(central_europe, || {
            let local = DateTime::new(2026, 1, 1);
            let utc = local.to_utc();
            assert_eq!(local.compare_to(&utc), Ordering::Equal);
            assert_ne!(local, utc);
            assert_eq!(local.cmp(&utc), Ordering::Less);
            assert_eq!(local.cmp(&local.to_local()), Ordering::Equal);
            assert!(utc < local.add(Duration::from_micros(1)));
        });
    }

    #[test]
    fn add_keeps_whether_the_value_is_local() {
        with_zone(central_europe, || {
            let local = DateTime::new(2026, 1, 1);
            assert!(!local.add(Duration::from_secs(1)).is_utc());
            assert!(
                DateTime::utc(2026, 1, 1)
                    .add(Duration::from_secs(1))
                    .is_utc()
            );
        });
    }
}
