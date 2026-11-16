//! The local time zone as Linux's C library gives it: `localtime_r`'s `tm_gmtoff` and `tm_zone`,
//! after `tzset`, as the Dart VM's `os_linux.cc` reads them, since POSIX does not promise that
//! `localtime_r` reads a changed zone. Nothing is kept between calls.
//!
//! The `unsafe` is the C calls: `localtime_r` writes a `tm` it is handed, and `tm_zone` points
//! into the C library's own zone data.
#![allow(unsafe_code)]

use std::ffi::CStr;
use std::mem::MaybeUninit;

use waft_embedder::SignedDuration;

use super::Linux;
use crate::os::capabilities::LocalTimeZone;

unsafe extern "C" {
    /// POSIX's `tzset`, which the `libc` crate does not declare for Linux.
    fn tzset();
}

impl LocalTimeZone for Linux {
    #[allow(
        clippy::useless_conversion,
        reason = "`tm_gmtoff` is a C `long`, 32 bits on a 32-bit target"
    )]
    fn time_zone_offset(seconds_since_epoch: i64) -> SignedDuration {
        SignedDuration::from_seconds(
            local_time(seconds_since_epoch).map_or(0, |tm| i64::from(tm.tm_gmtoff)),
        )
    }

    fn time_zone_name(seconds_since_epoch: i64) -> String {
        local_time(seconds_since_epoch)
            .filter(|tm| !tm.tm_zone.is_null())
            .map(|tm| {
                // SAFETY: a non-null `tm_zone` from `localtime_r` is a NUL-terminated string in
                // the C library's zone data, which stays allocated; a later `tzset` may rewrite
                // it, so it is copied at once, as Dart's `date.cc` copies it.
                unsafe { CStr::from_ptr(tm.tm_zone) }
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_default()
    }
}

/// The broken-down local time at `seconds_since_epoch`, or `None` if the C library cannot
/// represent it (Dart answers zero and an empty name then), as a 32-bit `time_t` cannot
/// represent every instant.
#[allow(
    clippy::useless_conversion,
    reason = "`time_t` is 32 bits on a 32-bit target"
)]
fn local_time(seconds_since_epoch: i64) -> Option<libc::tm> {
    let seconds = libc::time_t::try_from(seconds_since_epoch).ok()?;
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
