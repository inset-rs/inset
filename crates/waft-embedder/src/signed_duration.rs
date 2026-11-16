//! dart:core `Duration`, where it can be negative: a time zone's offset west of UTC, the
//! difference between two instants. Everywhere else Waft takes `std::time::Duration`, which Dart's
//! non-negative durations map to.

use std::fmt;
use std::ops::{Add, Neg, Sub};

/// A span of time, such as 27 days, 4 hours, 12 minutes and 3 seconds, which may be negative.
///
/// Dart's `Duration`, kept beside `std::time::Duration` under its own name: the framework's
/// durations are never negative and take std's, and a name shared with std's would shadow it in
/// every `use waft::*`.
///
/// ```
/// # use waft_embedder::SignedDuration;
/// let west = SignedDuration::from_hours(-8);
/// assert_eq!(west.in_minutes(), -480);
/// assert_eq!(west.to_string(), "-8:00:00.000000");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SignedDuration {
    microseconds: i64,
}

const MICROSECONDS_PER_MILLISECOND: i64 = 1000;
const MICROSECONDS_PER_SECOND: i64 = 1_000_000;
const MICROSECONDS_PER_MINUTE: i64 = 60 * MICROSECONDS_PER_SECOND;
const MICROSECONDS_PER_HOUR: i64 = 60 * MICROSECONDS_PER_MINUTE;
const MICROSECONDS_PER_DAY: i64 = 24 * MICROSECONDS_PER_HOUR;

impl SignedDuration {
    /// An empty duration, representing zero time.
    pub const ZERO: SignedDuration = SignedDuration { microseconds: 0 };

    /// Dart's `Duration(days: ..)`.
    pub const fn from_days(days: i64) -> SignedDuration {
        SignedDuration::from_microseconds(days * MICROSECONDS_PER_DAY)
    }

    /// Dart's `Duration(hours: ..)`.
    pub const fn from_hours(hours: i64) -> SignedDuration {
        SignedDuration::from_microseconds(hours * MICROSECONDS_PER_HOUR)
    }

    /// Dart's `Duration(minutes: ..)`.
    pub const fn from_minutes(minutes: i64) -> SignedDuration {
        SignedDuration::from_microseconds(minutes * MICROSECONDS_PER_MINUTE)
    }

    /// Dart's `Duration(seconds: ..)`.
    pub const fn from_seconds(seconds: i64) -> SignedDuration {
        SignedDuration::from_microseconds(seconds * MICROSECONDS_PER_SECOND)
    }

    /// Dart's `Duration(milliseconds: ..)`.
    pub const fn from_milliseconds(milliseconds: i64) -> SignedDuration {
        SignedDuration::from_microseconds(milliseconds * MICROSECONDS_PER_MILLISECOND)
    }

    /// Dart's `Duration(microseconds: ..)`.
    pub const fn from_microseconds(microseconds: i64) -> SignedDuration {
        SignedDuration { microseconds }
    }

    /// The number of entire days spanned by this duration, truncated toward zero.
    pub const fn in_days(&self) -> i64 {
        self.microseconds / MICROSECONDS_PER_DAY
    }

    /// The number of entire hours spanned by this duration, truncated toward zero.
    pub const fn in_hours(&self) -> i64 {
        self.microseconds / MICROSECONDS_PER_HOUR
    }

    /// The number of entire minutes spanned by this duration, truncated toward zero.
    pub const fn in_minutes(&self) -> i64 {
        self.microseconds / MICROSECONDS_PER_MINUTE
    }

    /// The number of entire seconds spanned by this duration, truncated toward zero.
    pub const fn in_seconds(&self) -> i64 {
        self.microseconds / MICROSECONDS_PER_SECOND
    }

    /// The number of entire milliseconds spanned by this duration, truncated toward zero.
    pub const fn in_milliseconds(&self) -> i64 {
        self.microseconds / MICROSECONDS_PER_MILLISECOND
    }

    /// The number of whole microseconds spanned by this duration.
    pub const fn in_microseconds(&self) -> i64 {
        self.microseconds
    }

    /// Whether this duration is negative.
    pub const fn is_negative(&self) -> bool {
        self.microseconds < 0
    }

    /// A new duration with the same length as this one, but positive.
    pub const fn abs(&self) -> SignedDuration {
        SignedDuration::from_microseconds(self.microseconds.abs())
    }
}

impl Neg for SignedDuration {
    type Output = SignedDuration;

    fn neg(self) -> SignedDuration {
        SignedDuration::from_microseconds(-self.microseconds)
    }
}

impl Add for SignedDuration {
    type Output = SignedDuration;

    fn add(self, other: SignedDuration) -> SignedDuration {
        SignedDuration::from_microseconds(self.microseconds + other.microseconds)
    }
}

impl Sub for SignedDuration {
    type Output = SignedDuration;

    fn sub(self, other: SignedDuration) -> SignedDuration {
        SignedDuration::from_microseconds(self.microseconds - other.microseconds)
    }
}

/// A non-negative std duration as a signed one, as every Dart `Duration` the framework takes is.
impl From<std::time::Duration> for SignedDuration {
    fn from(duration: std::time::Duration) -> SignedDuration {
        SignedDuration::from_microseconds(
            i64::try_from(duration.as_micros()).expect("a duration within range"),
        )
    }
}

/// Dart's `Duration.toString`: hours, minutes, seconds and microseconds, as `-8:00:00.000000`.
impl fmt::Display for SignedDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.is_negative() { "-" } else { "" };
        let microseconds = self.microseconds.unsigned_abs();
        let per_hour = MICROSECONDS_PER_HOUR.unsigned_abs();
        let per_minute = MICROSECONDS_PER_MINUTE.unsigned_abs();
        let per_second = MICROSECONDS_PER_SECOND.unsigned_abs();
        let hours = microseconds / per_hour;
        let minutes = microseconds % per_hour / per_minute;
        let seconds = microseconds % per_minute / per_second;
        let micros = microseconds % per_second;
        write!(f, "{sign}{hours}:{minutes:02}:{seconds:02}.{micros:06}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `core/duration_test.dart`'s `toString` cases.
    #[test]
    fn to_string_follows_dart() {
        assert_eq!(SignedDuration::ZERO.to_string(), "0:00:00.000000");
        assert_eq!(SignedDuration::from_hours(1).to_string(), "1:00:00.000000");
        assert_eq!(
            (SignedDuration::from_hours(-8) - SignedDuration::from_minutes(30)).to_string(),
            "-8:30:00.000000"
        );
        assert_eq!(
            SignedDuration::from_microseconds(-1).to_string(),
            "-0:00:00.000001"
        );
        assert_eq!(SignedDuration::from_days(2).to_string(), "48:00:00.000000");
    }

    #[test]
    fn the_in_getters_truncate_toward_zero() {
        let d = SignedDuration::from_seconds(-90);
        assert_eq!((d.in_minutes(), d.in_seconds()), (-1, -90));
        assert_eq!(d.abs().in_minutes(), 1);
        assert_eq!(-d, SignedDuration::from_seconds(90));
        assert!(d.is_negative());
    }
}
