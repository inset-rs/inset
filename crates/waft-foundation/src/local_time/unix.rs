//! macOS, iOS, Linux and Android: `localtime_r` gives the offset in `tm_gmtoff` and the zone's
//! abbreviation in `tm_zone`, as the Dart VM's `os_macos.cc` and `os_linux.cc` read them.
//!
//! The `unsafe` is the C calls: `localtime_r` writes a `tm` it is handed, and `tm_zone` points
//! into the C library's own zone data.
#![allow(unsafe_code)]

use std::ffi::CStr;
use std::mem::MaybeUninit;

#[cfg(not(target_vendor = "apple"))]
unsafe extern "C" {
    /// POSIX's `tzset`, which the `libc` crate does not declare on these targets.
    fn tzset();
}

#[allow(
    clippy::useless_conversion,
    reason = "`tm_gmtoff` is a C `long`, 32 bits on a 32-bit target"
)]
pub(super) fn offset_in_seconds(seconds_since_epoch: i64) -> i64 {
    local_time(seconds_since_epoch).map_or(0, |tm| i64::from(tm.tm_gmtoff))
}

pub(super) fn zone_name(seconds_since_epoch: i64) -> String {
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
/// represent it (Dart answers zero and an empty name then).
#[allow(
    clippy::useless_conversion,
    reason = "`time_t` is 32 bits on a 32-bit Android target"
)]
fn local_time(seconds_since_epoch: i64) -> Option<libc::tm> {
    let seconds = libc::time_t::try_from(seconds_since_epoch).ok()?;
    // Dart's `os_linux.cc` calls `tzset` first, since POSIX does not promise that
    // `localtime_r` reads a changed zone; `os_macos.cc` does not need it.
    #[cfg(not(target_vendor = "apple"))]
    // SAFETY: `tzset` takes nothing and only updates the C library's zone state.
    unsafe {
        tzset();
    }
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
