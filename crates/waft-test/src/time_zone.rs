//! [`TestTimeZone`]: the local time zone a [`TestPlatform`](crate::TestPlatform) answers, so a
//! test of local time reads the zone it chose rather than the machine's.

use std::fmt;
use std::rc::Rc;

use waft_embedder::SignedDuration;

/// The offset from UTC and the zone's name at an instant in seconds since the epoch, as a host
/// answers `Platform::time_zone_offset` and `Platform::time_zone_name`.
type Answer = dyn Fn(i64) -> (SignedDuration, String);

/// A local time zone for a test: UTC, a fixed offset and name, or any rule of the instant, such
/// as a zone with summer time.
#[derive(Clone)]
pub struct TestTimeZone {
    answer: Rc<Answer>,
}

impl TestTimeZone {
    /// UTC, the zone a `TestPlatform` starts in.
    pub fn utc() -> TestTimeZone {
        TestTimeZone::fixed(SignedDuration::ZERO, "UTC")
    }

    /// A zone that is `offset` from UTC all year, named `name`.
    pub fn fixed(offset: SignedDuration, name: &str) -> TestTimeZone {
        let name = name.to_string();
        TestTimeZone::new(move |_| (offset, name.clone()))
    }

    /// A zone whose offset and name at an instant in seconds since the epoch `rule` answers.
    pub fn new(rule: impl Fn(i64) -> (SignedDuration, String) + 'static) -> TestTimeZone {
        TestTimeZone {
            answer: Rc::new(rule),
        }
    }

    /// The offset from UTC at `seconds_since_epoch`.
    pub fn offset(&self, seconds_since_epoch: i64) -> SignedDuration {
        (self.answer)(seconds_since_epoch).0
    }

    /// The zone's name at `seconds_since_epoch`.
    pub fn name(&self, seconds_since_epoch: i64) -> String {
        (self.answer)(seconds_since_epoch).1
    }
}

impl Default for TestTimeZone {
    fn default() -> TestTimeZone {
        TestTimeZone::utc()
    }
}

impl fmt::Debug for TestTimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestTimeZone").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use waft_embedder::SignedDuration;
    use waft_foundation::{AppCell, DateTime};

    use crate::{TestPlatform, TestTimeZone};

    #[test]
    fn a_test_platform_is_in_utc_until_a_case_sets_its_zone() {
        let platform = Rc::new(TestPlatform::new());
        let cell = AppCell::with_platform(Rc::clone(&platform) as waft_embedder::PlatformRef);
        let app = cell.borrow();
        let new_year = DateTime::new(&app, 2026, 1, 1);
        assert_eq!(new_year.time_zone_offset(&app), SignedDuration::ZERO);
        assert_eq!(new_year.time_zone_name(&app), "UTC");

        platform.set_time_zone(TestTimeZone::fixed(SignedDuration::from_hours(1), "CET"));
        let in_paris = DateTime::new(&app, 2026, 1, 1);
        assert_eq!(
            in_paris.time_zone_offset(&app),
            SignedDuration::from_hours(1)
        );
        assert_eq!(in_paris.to_utc().hour(), 23);
        assert_eq!(new_year.hour(), 0, "the fields are kept from creation");
        assert_eq!(
            new_year.time_zone_offset(&app),
            SignedDuration::from_hours(1),
            "asked again"
        );
        assert_eq!(new_year.time_zone_name(&app), "CET", "asked again");
    }
}
