//! Core Media's times beside the host's durations: a player's and an asset's times, and a seek's or
//! a picture's tolerance, which AVFoundation takes infinite where it is unbounded.

use std::time::Duration;

use objc2_core_media::{CMTime, CMTimeFlags, kCMTimePositiveInfinity};

/// The timescale a time this host gives AVFoundation is in: nanoseconds.
const NANOSECONDS: i32 = 1_000_000_000;

/// `time` as a duration; `None` for an invalid or indefinite time, as a live stream's duration
/// is.
pub(super) fn duration_of(time: CMTime) -> Option<Duration> {
    let usable =
        time.flags.contains(CMTimeFlags::Valid) && !time.flags.contains(CMTimeFlags::Indefinite);
    if !usable || time.timescale <= 0 {
        return None;
    }
    Duration::try_from_secs_f64(time.value as f64 / f64::from(time.timescale)).ok()
}

pub(super) fn cm_time_of(position: Duration) -> CMTime {
    // SAFETY: a plain value constructor.
    unsafe { CMTime::with_seconds(position.as_secs_f64(), NANOSECONDS) }
}

/// A tolerance on one side as AVFoundation takes it: infinite where it is unbounded.
pub(super) fn tolerance_time(tolerance: Duration) -> CMTime {
    if tolerance == Duration::MAX {
        // SAFETY: a constant Core Media exports for the life of the process.
        unsafe { kCMTimePositiveInfinity }
    } else {
        cm_time_of(tolerance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_is_a_duration_only_while_it_is_valid_and_definite() {
        let time = |value, flags| CMTime {
            value,
            timescale: 600,
            flags,
            epoch: 0,
        };
        assert_eq!(
            duration_of(time(900, CMTimeFlags::Valid)),
            Some(Duration::from_millis(1500))
        );
        assert_eq!(duration_of(time(900, CMTimeFlags::empty())), None);
        assert_eq!(
            duration_of(time(0, CMTimeFlags::Valid | CMTimeFlags::Indefinite)),
            None
        );
    }
}
