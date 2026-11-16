//! The local time zone as macOS and iOS give it, for `Platform::time_zone_offset` and
//! `Platform::time_zone_name`: `localtime_r`'s `tm_gmtoff` and `tm_zone`, as the Dart VM's
//! `os_macos.cc` reads them. Apple's C library reads a changed zone in `localtime_r` itself, so,
//! as in Dart, no `tzset` comes first. Nothing is kept between calls.
//!
//! The `unsafe` is the C call: `localtime_r` writes a `tm` it is handed, and `tm_zone` points into
//! the C library's own zone data.
#![allow(unsafe_code)]

use std::ffi::CStr;
use std::mem::MaybeUninit;

use waft_embedder::SignedDuration;

/// The local zone's offset from UTC at `seconds_since_epoch`, or zero where the C library cannot
/// represent the instant, as Dart answers then.
pub fn time_zone_offset(seconds_since_epoch: i64) -> SignedDuration {
    SignedDuration::from_seconds(local_time(seconds_since_epoch).map_or(0, |tm| tm.tm_gmtoff))
}

/// The local zone's abbreviation at `seconds_since_epoch`, such as "CET", or empty where the C
/// library cannot represent the instant.
pub fn time_zone_name(seconds_since_epoch: i64) -> String {
    local_time(seconds_since_epoch)
        .filter(|tm| !tm.tm_zone.is_null())
        .map(|tm| {
            // SAFETY: a non-null `tm_zone` from `localtime_r` is a NUL-terminated string in the
            // C library's zone data, which stays allocated; a later reload of the zone may
            // rewrite it, so it is copied at once, as Dart's `date.cc` copies it.
            unsafe { CStr::from_ptr(tm.tm_zone) }
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_default()
}

/// The broken-down local time at `seconds_since_epoch`, or `None` if the C library cannot
/// represent it.
fn local_time(seconds_since_epoch: i64) -> Option<libc::tm> {
    let seconds: libc::time_t = seconds_since_epoch;
    let mut tm = MaybeUninit::<libc::tm>::zeroed();
    // SAFETY: both pointers are valid for the call; `localtime_r` fills `tm` and returns it,
    // or null on failure.
    let filled = unsafe { libc::localtime_r(&seconds, tm.as_mut_ptr()) };
    if filled.is_null() {
        return None;
    }
    // SAFETY: `localtime_r` succeeded, so it initialized `tm`.
    Some(unsafe { tm.assume_init() })
}

#[cfg(test)]
mod tests {
    /// The system answers a plausible offset for now: within a day of UTC, in whole seconds.
    #[test]
    fn the_system_gives_a_plausible_offset_and_a_name() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_secs();
        let now = i64::try_from(now).expect("within range");
        let offset = super::time_zone_offset(now).in_seconds();
        assert!(offset.abs() < 24 * 60 * 60, "{offset}");
        assert!(!super::time_zone_name(now).is_empty());
    }
}
