//! Inside a WASI 0.3 component: the host's `wasi:clocks/timezone@0.3.0`, imported through
//! Waft's own bindings of that one interface (`wit/world.wit`), as standalone dart2wasm imports
//! its two calls from its embedder. The interface gives an IANA id for a name, since WASI has no
//! abbreviation; a host with a zone but no IANA id is named by its debug string, shown and never
//! parsed; a host that gives no zone reads as UTC.
//!
//! The `unsafe` is the canonical ABI glue `wit-bindgen` generates for the two imports.
#![allow(unsafe_code)]

wit_bindgen::generate!({
    path: "wit",
    world: "local-time",
    features: ["clocks-timezone"],
    generate_all,
});

use wasi::clocks::system_clock::Instant;
use wasi::clocks::timezone;

const NANOSECONDS_PER_SECOND: i64 = 1_000_000_000;

pub(super) fn offset_in_seconds(seconds_since_epoch: i64) -> i64 {
    let when = Instant {
        seconds: seconds_since_epoch,
        nanoseconds: 0,
    };
    timezone::utc_offset(when).map_or(0, |nanoseconds| nanoseconds / NANOSECONDS_PER_SECOND)
}

pub(super) fn zone_name(seconds_since_epoch: i64) -> String {
    if let Some(iana_id) = timezone::iana_id() {
        return iana_id;
    }
    let when = Instant {
        seconds: seconds_since_epoch,
        nanoseconds: 0,
    };
    // A zone with no IANA id still needs a name beside its offset; WASI's only other text for
    // it is the debug string, which is shown here and never parsed.
    if timezone::utc_offset(when).is_some() {
        return timezone::to_debug_string();
    }
    "UTC".to_string()
}
