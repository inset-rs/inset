//! The local time zone, as the Dart VM's two natives give it to `DateTime`: the offset from UTC
//! and the zone's name at an instant, asked of the system on every call with nothing cached, so
//! a change of zone is seen at the next call. One backend per target, as Dart's patch files are
//! one per backend:
//!
//! - `unix`: `localtime_r`'s `tm_gmtoff` and `tm_zone` (macOS, iOS, Linux, Android);
//! - `windows`: the zone's rules for the year from Win32;
//! - `web`: JS `Date`, as dart2js asks it;
//! - `wasi`: WASI 0.3's `wasi:clocks/timezone`, inside a component;
//! - `utc` for any other target, which has no zone to ask.
//!
//! The instant is in whole seconds, already mapped into the range a 32-bit `time_t` holds
//! (`DateTime`'s equivalent-year mapping), as Dart's natives take it.
//!
//! A test swaps the system for a fixed zone with `tests::with_zone`, since a test cannot depend
//! on the machine's.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as system;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as system;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web as system;

#[cfg(all(target_os = "wasi", target_env = "p3"))]
mod wasi;
#[cfg(all(target_os = "wasi", target_env = "p3"))]
use wasi as system;

#[cfg(not(any(
    unix,
    windows,
    all(target_arch = "wasm32", target_os = "unknown"),
    all(target_os = "wasi", target_env = "p3")
)))]
mod utc;
#[cfg(not(any(
    unix,
    windows,
    all(target_arch = "wasm32", target_os = "unknown"),
    all(target_os = "wasi", target_env = "p3")
)))]
use utc as system;

/// The local time's offset from UTC at `seconds_since_epoch`, in seconds, positive east of UTC:
/// Dart's `DateTime_timeZoneOffsetInSeconds`.
pub(crate) fn offset_in_seconds(seconds_since_epoch: i64) -> i64 {
    #[cfg(test)]
    if let Some(zone) = tests::fixed_zone() {
        return zone(seconds_since_epoch).0;
    }
    system::offset_in_seconds(seconds_since_epoch)
}

/// The local zone's name at `seconds_since_epoch`, as the system gives it: Dart's
/// `DateTime_timeZoneName`.
pub(crate) fn zone_name(seconds_since_epoch: i64) -> String {
    #[cfg(test)]
    if let Some(zone) = tests::fixed_zone() {
        return zone(seconds_since_epoch).1.to_string();
    }
    system::zone_name(seconds_since_epoch)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;

    /// A zone for a test: the offset in seconds and the name at an instant.
    pub(crate) type Zone = fn(i64) -> (i64, &'static str);

    thread_local! {
        static FIXED: Cell<Option<Zone>> = const { Cell::new(None) };
    }

    pub(super) fn fixed_zone() -> Option<Zone> {
        FIXED.with(Cell::get)
    }

    /// Runs `body` with `zone` standing in for the system's time zone on this thread.
    pub(crate) fn with_zone<R>(zone: Zone, body: impl FnOnce() -> R) -> R {
        struct Restore(Option<Zone>);
        impl Drop for Restore {
            fn drop(&mut self) {
                FIXED.with(|fixed| fixed.set(self.0));
            }
        }
        let _restore = Restore(FIXED.with(|fixed| fixed.replace(Some(zone))));
        body()
    }

    /// The native backend answers a plausible offset and a name for now: within a day of UTC,
    /// in whole minutes or seconds as every zone is.
    #[test]
    fn the_system_gives_a_plausible_offset_and_a_name() {
        let now = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_secs();
        let now = i64::try_from(now).expect("within range");
        let offset = super::system::offset_in_seconds(now);
        assert!(offset.abs() < 24 * 60 * 60, "{offset}");
        let _name = super::system::zone_name(now);
    }
}
