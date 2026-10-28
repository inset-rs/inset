//! The key-downs AppKit delivered lately, each kept by the key the host reported for it, so that a
//! key an app kept back and leaves later goes on to the menu bar as the system's own event, with
//! its keyboard layout and modifiers (`key_equivalents`). An app that answers a key later than
//! AppKit lets it wait, as WaOS answers for a program on its own thread, hands the key back by its
//! `KeyData`, as Chrome's browser process keeps the event its renderer has not answered yet.
//!
//! The invariant: what is kept is bounded, by count and by age, so a key no app hands back is
//! let go; and a key is given back once at most.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use inset_embedder::KeyData;

/// The most key-downs kept: a few, as an app hands a key back within moments.
const MOST: usize = 16;
/// The longest a key-down is kept: a shortcut that comes later than this would surprise the user,
/// who has moved on.
const LONGEST: Duration = Duration::from_secs(3);

/// Events of key-downs, by the key the host reported for each.
pub(super) struct KeptKeys<E> {
    kept: VecDeque<Kept<E>>,
}

struct Kept<E> {
    time_stamp: Duration,
    physical: u64,
    kept_at: Instant,
    event: E,
}

impl<E> KeptKeys<E> {
    pub(super) const fn new() -> KeptKeys<E> {
        KeptKeys {
            kept: VecDeque::new(),
        }
    }

    /// Keeps `event`, the system's own of `key`, letting go of the oldest beyond the bounds.
    pub(super) fn keep(&mut self, key: &KeyData, event: E, now: Instant) {
        self.let_go_of_expired(now);
        if self.kept.len() == MOST {
            self.kept.pop_front();
        }
        self.kept.push_back(Kept {
            time_stamp: key.time_stamp,
            physical: key.physical,
            kept_at: now,
            event,
        });
    }

    /// The event kept for `key`, given back once; `None` once it is gone.
    pub(super) fn take(&mut self, key: &KeyData, now: Instant) -> Option<E> {
        self.let_go_of_expired(now);
        let index = self
            .kept
            .iter()
            .position(|kept| kept.time_stamp == key.time_stamp && kept.physical == key.physical)?;
        self.kept.remove(index).map(|kept| kept.event)
    }

    fn let_go_of_expired(&mut self, now: Instant) {
        self.kept
            .retain(|kept| now.saturating_duration_since(kept.kept_at) <= LONGEST);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(at_millis: u64) -> KeyData {
        KeyData {
            time_stamp: Duration::from_millis(at_millis),
            physical: 0x0007_0012,
            logical: 0x6f,
            ..KeyData::default()
        }
    }

    #[test]
    fn a_key_is_given_back_once_by_its_data() {
        let now = Instant::now();
        let mut kept = KeptKeys::new();
        kept.keep(&key(1), "first", now);
        kept.keep(&key(2), "second", now);
        assert_eq!(kept.take(&key(2), now), Some("second"));
        assert_eq!(kept.take(&key(2), now), None, "given back already");
        assert_eq!(kept.take(&key(3), now), None, "never kept");
        assert_eq!(kept.take(&key(1), now), Some("first"));
    }

    #[test]
    fn a_key_is_let_go_of_past_the_bounds() {
        let now = Instant::now();
        let mut kept = KeptKeys::new();
        kept.keep(&key(0), 0, now);
        assert_eq!(
            kept.take(&key(0), now + LONGEST + Duration::from_millis(1)),
            None
        );
        for at in 0..=MOST as u64 {
            kept.keep(&key(at), at, now);
        }
        assert_eq!(kept.take(&key(0), now), None, "the oldest beyond the count");
        assert_eq!(kept.take(&key(MOST as u64), now), Some(MOST as u64));
    }
}
