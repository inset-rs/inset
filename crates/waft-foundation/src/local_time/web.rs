//! The web: JS `Date` gives the offset (`getTimezoneOffset`, in minutes west of UTC) and the
//! zone's name (the text in parentheses of `toString`), as dart2js and dart2wasm ask them
//! (`_internal/wasm/js_common/date_patch_patch.dart`).

use js_sys::Date;
use js_sys::wasm_bindgen::JsValue;

pub(super) fn offset_in_seconds(seconds_since_epoch: i64) -> i64 {
    // Minutes to seconds before the truncation, as Dart's: an old zone's offset can have
    // seconds, which `getTimezoneOffset` gives as a fraction of a minute.
    -((date(seconds_since_epoch).get_timezone_offset() * 60.0) as i64)
}

pub(super) fn zone_name(seconds_since_epoch: i64) -> String {
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
