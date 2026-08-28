//! Flutter counterpart: `scheduler/priority.dart`.

use std::ops::{Add, Sub};

/// A task priority, as passed to `SchedulerBinding.scheduleTask`.
#[derive(Clone, Copy)]
pub struct Priority {
    value: i64,
}

impl Priority {
    const fn new(value: i64) -> Priority {
        Priority { value }
    }

    /// Integer that describes this Priority value.
    pub fn value(self) -> i64 {
        self.value
    }

    /// A task to run after all other tasks, when no animations are running.
    pub const IDLE: Priority = Priority::new(0);

    /// A task to run even when animations are running.
    pub const ANIMATION: Priority = Priority::new(100000);

    /// A task to run even when the user is interacting with the device.
    pub const TOUCH: Priority = Priority::new(200000);

    /// Maximum offset by which to clamp relative priorities.
    ///
    /// It is still possible to have priorities that are offset by more than
    /// this amount by repeatedly taking relative offsets, but that is generally
    /// discouraged.
    pub const K_MAX_OFFSET: i64 = 10000;
}

impl Add<i64> for Priority {
    type Output = Priority;

    /// Returns a priority relative to this priority.
    ///
    /// A positive `offset` indicates a higher priority.
    ///
    /// The parameter `offset` is clamped to ±[`Priority::K_MAX_OFFSET`].
    fn add(self, mut offset: i64) -> Priority {
        if offset.abs() > Priority::K_MAX_OFFSET {
            // Clamp the input offset.
            offset = Priority::K_MAX_OFFSET * offset.signum();
        }
        Priority::new(self.value + offset)
    }
}

impl Sub<i64> for Priority {
    type Output = Priority;

    /// Returns a priority relative to this priority.
    ///
    /// A positive offset indicates a lower priority.
    ///
    /// The parameter `offset` is clamped to ±[`Priority::K_MAX_OFFSET`].
    fn sub(self, offset: i64) -> Priority {
        self + (-offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constants_carry_flutters_values() {
        assert_eq!(Priority::IDLE.value(), 0);
        assert_eq!(Priority::ANIMATION.value(), 100000);
        assert_eq!(Priority::TOUCH.value(), 200000);
    }

    // priority_test.dart 'Priority operators control test'
    #[test]
    fn offsets_are_clamped_to_the_maximum() {
        assert_eq!(
            (Priority::IDLE + (Priority::K_MAX_OFFSET + 100)).value(),
            Priority::IDLE.value() + Priority::K_MAX_OFFSET
        );
        assert_eq!(
            (Priority::ANIMATION - (Priority::K_MAX_OFFSET + 100)).value(),
            Priority::ANIMATION.value() - Priority::K_MAX_OFFSET
        );
        assert_eq!((Priority::IDLE + 100).value(), 100);
        assert_eq!((Priority::ANIMATION - 5).value(), 99995);
    }
}
