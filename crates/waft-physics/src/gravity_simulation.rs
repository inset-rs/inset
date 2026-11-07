//! Flutter counterpart: `physics/gravity_simulation.dart`.

use std::fmt::{self, Debug};

use crate::{Simulation, Tolerance};

/// A simulation that applies a constant accelerating force.
///
/// Models a particle that follows Newton's second law of motion. The simulation
/// ends when the position exceeds a defined threshold.
pub struct GravitySimulation {
    x0: f64,
    v0: f64,
    a: f64,
    end: f64,
    /// How close to the actual end of the simulation a value at a particular
    /// time must be before [`is_done`](Simulation::is_done) considers the
    /// simulation to be "done".
    pub tolerance: Tolerance,
}

impl GravitySimulation {
    /// Creates a [`GravitySimulation`] using the given arguments, which are,
    /// respectively: an acceleration that is to be applied continually over
    /// time; an initial position relative to an origin; the magnitude of the
    /// distance from that origin beyond which (in either direction) to consider
    /// the simulation to be "done", which must be positive; and an initial
    /// velocity.
    ///
    /// The initial position and maximum distance are measured in arbitrary
    /// length units L from an arbitrary origin. The units will match those used
    /// for [`x`](Simulation::x).
    ///
    /// The time unit T used for the arguments to [`x`](Simulation::x),
    /// [`dx`](Simulation::dx), and [`is_done`](Simulation::is_done), combined
    /// with the aforementioned length unit, together determine the units that
    /// must be used for the velocity and acceleration arguments: L/T and L/T²
    /// respectively. The same units of velocity are used for the velocity
    /// obtained from [`dx`](Simulation::dx).
    pub fn new(
        acceleration: f64,
        distance: f64,
        end_distance: f64,
        velocity: f64,
    ) -> GravitySimulation {
        debug_assert!(end_distance >= 0.0);
        GravitySimulation {
            x0: distance,
            v0: velocity,
            a: acceleration,
            end: end_distance,
            tolerance: Tolerance::DEFAULT_TOLERANCE,
        }
    }
}

impl Simulation for GravitySimulation {
    fn x(&self, time: f64) -> f64 {
        self.x0 + self.v0 * time + 0.5 * self.a * time * time
    }

    fn dx(&self, time: f64) -> f64 {
        self.v0 + time * self.a
    }

    fn is_done(&self, time: f64) -> bool {
        self.x(time).abs() >= self.end
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

impl Debug for GravitySimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GravitySimulation(g: {:.1}, x₀: {:.1}, dx₀: {:.1}, xₘₐₓ: ±{:.1})",
            self.a, self.x0, self.v0, self.end
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Simulation;

    fn approx(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "{actual} != {expected} ±{epsilon}"
        );
    }

    #[test]
    fn gravity_simulation_1() {
        let gravity = GravitySimulation::new(9.81, 10.0, 0.0, 0.0);
        let text = format!("{gravity:?}");
        assert!(!text.contains('\n'));
        approx(gravity.x(10.0), 50.0 * 9.81 + 10.0, 1e-10);
    }

    #[test]
    fn gravity_simulation_2() {
        let gravity = GravitySimulation::new(-10.0, 0.0, 6.0, 10.0);

        assert_eq!(gravity.x(0.0), 0.0);
        assert_eq!(gravity.dx(0.0), 10.0);
        assert!(!gravity.is_done(0.0));

        assert_eq!(gravity.x(1.0), 5.0);
        assert_eq!(gravity.dx(1.0), 0.0);
        assert!(!gravity.is_done(0.2));

        assert_eq!(gravity.x(2.0), 0.0);
        assert_eq!(gravity.dx(2.0), -10.0);
        assert!(!gravity.is_done(2.0));

        assert_eq!(gravity.x(3.0), -15.0);
        assert_eq!(gravity.dx(3.0), -20.0);
        assert!(gravity.is_done(3.0));
    }

    #[test]
    fn test_gravity() {
        let gravity = GravitySimulation::new(200.0, 100.0, 600.0, 0.0);

        assert!(!gravity.is_done(0.0));
        assert_eq!(gravity.x(0.0), 100.0);
        assert_eq!(gravity.dx(0.0), 0.0);

        assert_eq!(gravity.x(0.25), 106.25);
        assert_eq!(gravity.x(0.50), 125.0);
        assert_eq!(gravity.x(0.75), 156.25);
        assert_eq!(gravity.x(1.00), 200.0);
        assert_eq!(gravity.x(1.25), 256.25);
        assert_eq!(gravity.x(1.50), 325.0);
        assert_eq!(gravity.x(1.75), 406.25);

        assert_eq!(gravity.dx(0.25), 50.0);
        assert_eq!(gravity.dx(0.50), 100.0);
        assert_eq!(gravity.dx(0.75), 150.00);
        assert_eq!(gravity.dx(1.00), 200.0);
        assert_eq!(gravity.dx(1.25), 250.0);
        assert_eq!(gravity.dx(1.50), 300.0);
        assert_eq!(gravity.dx(1.75), 350.0);

        assert!(gravity.is_done(2.5));
        assert_eq!(gravity.x(2.5), 725.0);
        assert_eq!(gravity.dx(2.5), 500.0);
    }
}
