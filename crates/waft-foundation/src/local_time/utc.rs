//! A target with no time zone to ask: local time is UTC.

pub(super) fn offset_in_seconds(_seconds_since_epoch: i64) -> i64 {
    0
}

pub(super) fn zone_name(_seconds_since_epoch: i64) -> String {
    "UTC".to_string()
}
