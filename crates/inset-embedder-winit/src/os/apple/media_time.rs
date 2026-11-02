//! Core Animation's clock beside the host's: a display link and a player speak in media time,
//! `CACurrentMediaTime`'s seconds, and the host in `Instant`s, so a refresh's target time crosses
//! from one to the other here.

use std::time::Instant;

use objc2_quartz_core::CACurrentMediaTime;

/// The moment `media_time`, on Core Animation's clock, is on the host's: a macOS display link's
/// target time.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn instant_of(media_time: f64) -> Instant {
    use std::time::Duration;

    let now = Instant::now();
    let from_now = media_time - CACurrentMediaTime();
    if from_now >= 0.0 {
        now + Duration::from_secs_f64(from_now)
    } else {
        now.checked_sub(Duration::from_secs_f64(-from_now))
            .unwrap_or(now)
    }
}

/// The moment `instant`, on the host's clock, is on Core Animation's.
pub(crate) fn media_time_of(instant: Instant) -> f64 {
    let now = Instant::now();
    let media_now = CACurrentMediaTime();
    match instant.checked_duration_since(now) {
        Some(ahead) => media_now + ahead.as_secs_f64(),
        None => media_now - now.duration_since(instant).as_secs_f64(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// A refresh a frame ahead, crossed to media time and back, lands where it was, to well
    /// under a refresh.
    #[test]
    fn a_time_crosses_between_the_clocks_and_back() {
        let ahead = Instant::now() + Duration::from_millis(16);
        let back = instant_of(media_time_of(ahead));
        let error = back
            .checked_duration_since(ahead)
            .unwrap_or_else(|| ahead.duration_since(back));
        assert!(error < Duration::from_millis(1), "{error:?}");
        let behind = Instant::now() - Duration::from_millis(16);
        assert!(media_time_of(behind) < CACurrentMediaTime());
    }
}
