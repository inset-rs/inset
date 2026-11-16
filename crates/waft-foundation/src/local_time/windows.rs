//! Windows: the zone's rules for the instant's year (`GetTimeZoneInformationForYear`) turn the
//! UTC time into local time (`SystemTimeToTzSpecificLocalTime`), and the zone's own standard or
//! daylight name names it, as the Dart VM's `os_win.cc` does.
//!
//! The `unsafe` is the Win32 calls, each filling a struct it is handed.
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows_sys::Win32::System::Time::{
    FileTimeToSystemTime, GetTimeZoneInformationForYear, SystemTimeToFileTime,
    SystemTimeToTzSpecificLocalTime, TIME_ZONE_INFORMATION,
};

/// Seconds from 1601-01-01, the `FILETIME` epoch, to 1970-01-01.
const SECONDS_FROM_1601_TO_1970: i64 = 11_644_473_600;
/// `FILETIME` ticks, of 100 ns, in a second.
const TICKS_PER_SECOND: i64 = 10_000_000;

pub(super) fn offset_in_seconds(seconds_since_epoch: i64) -> i64 {
    local(seconds_since_epoch).map_or(0, |local| local.offset_in_seconds)
}

pub(super) fn zone_name(seconds_since_epoch: i64) -> String {
    let Some(local) = local(seconds_since_epoch) else {
        return String::new();
    };
    let zone = &local.zone;
    // The daylight name while the daylight bias applies, as Dart's `os_win.cc` picks it.
    let daylight_offset = -i64::from(zone.Bias + zone.DaylightBias) * 60;
    let name = if zone.DaylightDate.wMonth != 0 && local.offset_in_seconds == daylight_offset {
        &zone.DaylightName
    } else {
        &zone.StandardName
    };
    let end = name
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(name.len());
    String::from_utf16_lossy(&name[..end])
}

struct Local {
    offset_in_seconds: i64,
    zone: TIME_ZONE_INFORMATION,
}

/// The zone's rules for the instant's year and the offset they give it, or `None` if Windows
/// cannot represent the instant.
fn local(seconds_since_epoch: i64) -> Option<Local> {
    let utc_ticks =
        (seconds_since_epoch + SECONDS_FROM_1601_TO_1970).checked_mul(TICKS_PER_SECOND)?;
    let utc_file_time = file_time(u64::try_from(utc_ticks).ok()?);
    let mut utc = SYSTEMTIME::default();
    let mut zone = TIME_ZONE_INFORMATION::default();
    let mut local = SYSTEMTIME::default();
    let mut local_file_time = FILETIME::default();
    // SAFETY: every pointer is to a local value valid for the call.
    let converted = unsafe {
        FileTimeToSystemTime(&utc_file_time, &mut utc) != 0
            && GetTimeZoneInformationForYear(utc.wYear, std::ptr::null(), &mut zone) != 0
            && SystemTimeToTzSpecificLocalTime(&zone, &utc, &mut local) != 0
            && SystemTimeToFileTime(&local, &mut local_file_time) != 0
    };
    if !converted {
        return None;
    }
    let local_ticks = i64::try_from(ticks(local_file_time)).ok()?;
    Some(Local {
        offset_in_seconds: (local_ticks - utc_ticks) / TICKS_PER_SECOND,
        zone,
    })
}

fn file_time(ticks: u64) -> FILETIME {
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

fn ticks(file_time: FILETIME) -> u64 {
    (u64::from(file_time.dwHighDateTime) << 32) | u64::from(file_time.dwLowDateTime)
}
