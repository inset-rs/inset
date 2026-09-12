//! Flutter counterpart: `physics/tolerance.dart`.

use std::fmt::{self, Debug};

/// Structure that specifies maximum allowable magnitudes for distances,
/// durations, and velocity differences to be considered equal.
#[derive(Clone, Copy)]
pub struct Tolerance {
    /// The magnitude of the maximum distance between two points for them to be
    /// considered within tolerance.
    ///
    /// The units for the distance tolerance must be the same as the units used
    /// for the distances that are to be compared to this tolerance.
    pub distance: f64,

    /// The magnitude of the maximum duration between two times for them to be
    /// considered within tolerance.
    ///
    /// The units for the time tolerance must be the same as the units used
    /// for the times that are to be compared to this tolerance.
    pub time: f64,

    /// The magnitude of the maximum difference between two velocities for them
    /// to be considered within tolerance.
    ///
    /// The units for the velocity tolerance must be the same as the units used
    /// for the velocities that are to be compared to this tolerance.
    pub velocity: f64,
}

const EPSILON_DEFAULT: f64 = 1e-3;

impl Tolerance {
    /// Creates a [`Tolerance`] object. By default, the distance, time, and
    /// velocity tolerances are all ±0.001; the constructor arguments override
    /// this.
    ///
    /// The arguments should all be positive values.
    pub const fn new(distance: f64, time: f64, velocity: f64) -> Tolerance {
        Tolerance {
            distance,
            time,
            velocity,
        }
    }

    /// A default tolerance of 0.001 for all three values.
    pub const DEFAULT_TOLERANCE: Tolerance = Tolerance {
        distance: EPSILON_DEFAULT,
        time: EPSILON_DEFAULT,
        velocity: EPSILON_DEFAULT,
    };
}

impl Default for Tolerance {
    fn default() -> Tolerance {
        Tolerance::DEFAULT_TOLERANCE
    }
}

impl Debug for Tolerance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tolerance(distance: ±{}, time: ±{}, velocity: ±{})",
            self.distance, self.time, self.velocity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_control_test() {
        let text = format!("{:?}", Tolerance::DEFAULT_TOLERANCE);
        assert!(!text.contains('\n'));
        assert!(!text.is_empty());
    }
}
