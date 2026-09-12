//! Flutter counterpart: `gestures/velocity_tracker.dart` (`Velocity` /
//! `VelocityEstimate`). The tracker class is in `velocity_tracker.rs`.

use std::fmt::{self, Debug, Display};
use std::ops::{Add, Neg, Sub};
use std::time::Duration;

use inset_embedder::Offset;

/// A velocity in two dimensions.
#[derive(Clone, Copy, PartialEq)]
pub struct Velocity {
    /// The number of pixels per second of velocity in the x and y directions.
    pub pixels_per_second: Offset,
}

impl Velocity {
    /// A velocity that isn't moving at all.
    pub const ZERO: Velocity = Velocity {
        pixels_per_second: Offset::ZERO,
    };

    /// Creates a [`Velocity`].
    pub const fn new(pixels_per_second: Offset) -> Velocity {
        Velocity { pixels_per_second }
    }

    /// Return a velocity whose magnitude has been clamped to `min_value`
    /// and `max_value`.
    pub fn clamp_magnitude(self, min_value: f64, max_value: f64) -> Velocity {
        debug_assert!(min_value >= 0.0);
        debug_assert!(max_value >= 0.0 && max_value >= min_value);
        let value_squared = self.pixels_per_second.distance_squared();
        if value_squared > max_value * max_value {
            return Velocity::new(
                (self.pixels_per_second / self.pixels_per_second.distance()) * max_value,
            );
        }
        if value_squared < min_value * min_value {
            return Velocity::new(
                (self.pixels_per_second / self.pixels_per_second.distance()) * min_value,
            );
        }
        self
    }
}

impl Neg for Velocity {
    type Output = Velocity;

    fn neg(self) -> Velocity {
        Velocity::new(-self.pixels_per_second)
    }
}

impl Sub for Velocity {
    type Output = Velocity;

    fn sub(self, other: Velocity) -> Velocity {
        Velocity::new(self.pixels_per_second - other.pixels_per_second)
    }
}

impl Add for Velocity {
    type Output = Velocity;

    fn add(self, other: Velocity) -> Velocity {
        Velocity::new(self.pixels_per_second + other.pixels_per_second)
    }
}

impl Display for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Velocity({:.1}, {:.1})",
            self.pixels_per_second.dx(),
            self.pixels_per_second.dy()
        )
    }
}

impl Debug for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// A two dimensional velocity estimate.
///
/// VelocityEstimates are computed by `VelocityTracker.getVelocityEstimate`.
pub struct VelocityEstimate {
    /// The number of pixels per second of velocity in the x and y directions.
    pub pixels_per_second: Offset,

    /// A value between 0.0 and 1.0 that indicates how well `VelocityTracker`
    /// was able to fit a straight line to its position data.
    pub confidence: f64,

    /// The time that elapsed between the first and last position sample used
    /// to compute [`pixels_per_second`](Self::pixels_per_second).
    pub duration: Duration,

    /// The difference between the first and last position sample used
    /// to compute [`pixels_per_second`](Self::pixels_per_second).
    pub offset: Offset,
}

impl VelocityEstimate {
    /// Creates a dimensional velocity estimate.
    pub fn new(
        pixels_per_second: Offset,
        confidence: f64,
        duration: Duration,
        offset: Offset,
    ) -> VelocityEstimate {
        VelocityEstimate {
            pixels_per_second,
            confidence,
            duration,
            offset,
        }
    }
}

impl Display for VelocityEstimate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VelocityEstimate({:.1}, {:.1}; offset: {:?}, duration: {:?}, confidence: {:.1})",
            self.pixels_per_second.dx(),
            self.pixels_per_second.dy(),
            self.offset,
            self.duration,
            self.confidence
        )
    }
}

impl Debug for VelocityEstimate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_magnitude_and_arithmetic() {
        let velocity = Velocity::new(Offset::new(3.0, 4.0));
        assert_eq!((-velocity).pixels_per_second, Offset::new(-3.0, -4.0));
        assert_eq!(
            (velocity + Velocity::new(Offset::new(1.0, 1.0))).pixels_per_second,
            Offset::new(4.0, 5.0)
        );
        let clamped = velocity.clamp_magnitude(10.0, 20.0);
        assert!((clamped.pixels_per_second.distance() - 10.0).abs() < 1e-9);
        assert_eq!(Velocity::ZERO.pixels_per_second, Offset::ZERO);
    }
}
