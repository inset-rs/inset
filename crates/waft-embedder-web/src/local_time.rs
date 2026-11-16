//! The local time zone as the browser gives it, for `Platform::time_zone_offset` and
//! `Platform::time_zone_name`: JS `Date`'s `getTimezoneOffset` (in minutes west of UTC) and the
//! name in parentheses of its `toString`, as dart2js and dart2wasm ask them
//! (`_internal/wasm/js_common/date_patch_patch.dart`). Nothing is kept between calls.

use js_sys::Date;
use js_sys::wasm_bindgen::JsValue;
use waft_embedder::SignedDuration;

/// The local zone's offset from UTC at `seconds_since_epoch`.
pub(crate) fn time_zone_offset(seconds_since_epoch: i64) -> SignedDuration {
    // Minutes to seconds before the truncation, as Dart's: an old zone's offset can have
    // seconds, which `getTimezoneOffset` gives as a fraction of a minute.
    let seconds_west = (date(seconds_since_epoch).get_timezone_offset() * 60.0) as i64;
    SignedDuration::from_seconds(-seconds_west)
}

/// The local zone's name at `seconds_since_epoch`, such as "Central European Standard Time".
pub(crate) fn time_zone_name(seconds_since_epoch: i64) -> String {
    let text = String::from(date(seconds_since_epoch).to_string());
    // `Thu Jan 01 1970 01:00:00 GMT+0100 (Central European Standard Time)`: from the first
    // `(` to the last `)`, as Dart's greedy `/\((.*)\)/`.
    match (text.find('('), text.rfind(')')) {
        (Some(open), Some(close)) if open < close => text[open + 1..close].to_string(),
        _ => String::new(),
    }
}

fn date(seconds_since_epoch: i64) -> Date {
    Date::new(&JsValue::from_f64(seconds_since_epoch as f64 * 1000.0))
}
