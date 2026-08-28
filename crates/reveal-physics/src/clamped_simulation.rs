//! Flutter counterpart: `physics/clamped_simulation.dart`.

use std::fmt::{self, Debug};

use reveal_geometry::clamp_double;

use crate::{Simulation, Tolerance};

/// A simulation that applies limits to another simulation.
///
/// The limits are only applied to the other simulation's outputs. For example,
/// if a maximum position was applied to a gravity simulation with the
/// particle's initial velocity being up, and the acceleration being down, and
/// the maximum position being between the initial position and the curve's
/// apogee, then the particle would return to its initial position in the same
/// amount of time as it would have if the maximum had not been applied; the
/// difference would just be that the position would be reported as pinned to
/// the maximum value for the times that it would otherwise have been reported
/// as higher.
///
/// Similarly, this means that the [`x`](Simulation::x) value will change at a
/// rate that does not match the reported [`dx`](Simulation::dx) value while
/// one or the other is being clamped.
///
/// The [`is_done`](Simulation::is_done) logic is unaffected by the clamping;
/// it reflects the logic of the underlying simulation.
pub struct ClampedSimulation {
    /// The simulation being clamped. Calls to [`x`](Simulation::x),
    /// [`dx`](Simulation::dx), and [`is_done`](Simulation::is_done) are
    /// forwarded to the simulation.
    pub simulation: Box<dyn Simulation>,
    /// The minimum to apply to [`x`](Simulation::x).
    pub x_min: f64,
    /// The maximum to apply to [`x`](Simulation::x).
    pub x_max: f64,
    /// The minimum to apply to [`dx`](Simulation::dx).
    pub dx_min: f64,
    /// The maximum to apply to [`dx`](Simulation::dx).
    pub dx_max: f64,
    /// How close to the actual end of the simulation a value at a particular
    /// time must be before [`is_done`](Simulation::is_done) considers the
    /// simulation to be "done".
    pub tolerance: Tolerance,
}

impl ClampedSimulation {
    /// Creates a [`ClampedSimulation`] that clamps the given simulation.
    ///
    /// The named arguments specify the ranges for the clamping behavior, as
    /// applied to [`x`](Simulation::x) and [`dx`](Simulation::dx).
    pub fn new(
        simulation: Box<dyn Simulation>,
        x_min: f64,
        x_max: f64,
        dx_min: f64,
        dx_max: f64,
    ) -> ClampedSimulation {
        debug_assert!(x_max >= x_min);
        debug_assert!(dx_max >= dx_min);
        ClampedSimulation {
            simulation,
            x_min,
            x_max,
            dx_min,
            dx_max,
            tolerance: Tolerance::DEFAULT_TOLERANCE,
        }
    }
}

impl Simulation for ClampedSimulation {
    fn x(&self, time: f64) -> f64 {
        clamp_double(self.simulation.x(time), self.x_min, self.x_max)
    }

    fn dx(&self, time: f64) -> f64 {
        clamp_double(self.simulation.dx(time), self.dx_min, self.dx_max)
    }

    fn is_done(&self, time: f64) -> bool {
        self.simulation.is_done(time)
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

impl Debug for ClampedSimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ClampedSimulation(simulation: {:?}, x: {:.1}..{:.1}, dx: {:.1}..{:.1})",
            self.simulation, self.x_min, self.x_max, self.dx_min, self.dx_max
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Simulation;
    use crate::gravity_simulation::GravitySimulation;

    struct TestSimulation;

    impl Simulation for TestSimulation {
        fn x(&self, _time: f64) -> f64 {
            0.0
        }

        fn dx(&self, _time: f64) -> f64 {
            0.0
        }

        fn is_done(&self, _time: f64) -> bool {
            true
        }

        fn tolerance(&self) -> Tolerance {
            Tolerance::DEFAULT_TOLERANCE
        }

        fn set_tolerance(&mut self, _tolerance: Tolerance) {}
    }

    impl Debug for TestSimulation {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestSimulation")
        }
    }

    #[test]
    fn clamped_simulation_1() {
        let gravity = GravitySimulation::new(9.81, 10.0, 0.0, 0.0);
        let clamped = ClampedSimulation::new(Box::new(gravity), 20.0, 100.0, 7.0, 11.0);

        assert_eq!(clamped.x(0.0), 20.0);
        assert_eq!(clamped.dx(0.0), 7.0);

        assert_eq!(clamped.x(100.0), 100.0);
        assert_eq!(clamped.dx(100.0), 11.0);
    }

    #[test]
    fn clamped_simulation_2() {
        let gravity = GravitySimulation::new(-10.0, 0.0, 6.0, 10.0);
        let clamped = ClampedSimulation::new(Box::new(gravity), 0.0, 2.5, -1.0, 1.0);

        assert_eq!(clamped.x(0.0), 0.0);
        assert_eq!(clamped.dx(0.0), 1.0);
        assert!(!clamped.is_done(0.0));

        assert_eq!(clamped.x(1.0), 2.5);
        assert_eq!(clamped.dx(1.0), 0.0);
        assert!(!clamped.is_done(0.2));

        assert_eq!(clamped.x(2.0), 0.0);
        assert_eq!(clamped.dx(2.0), -1.0);
        assert!(!clamped.is_done(2.0));

        assert_eq!(clamped.x(3.0), 0.0);
        assert_eq!(clamped.dx(3.0), -1.0);
        assert!(clamped.is_done(3.0));
    }

    #[test]
    fn simulation_to_string() {
        assert_eq!(
            format!(
                "{:?}",
                ClampedSimulation::new(Box::new(TestSimulation), -1.0, 2.0, -3.0, 4.0)
            ),
            "ClampedSimulation(simulation: TestSimulation, x: -1.0..2.0, dx: -3.0..4.0)"
        );
        assert_eq!(format!("{:?}", TestSimulation), "TestSimulation");
        assert_eq!(
            format!("{:?}", GravitySimulation::new(1.0, -2.0, 3.0, -4.0)),
            "GravitySimulation(g: 1.0, x₀: -2.0, dx₀: -4.0, xₘₐₓ: ±3.0)"
        );
        assert_eq!(
            format!("{:?}", crate::FrictionSimulation::new(1.0, -2.0, 3.0)),
            "FrictionSimulation(cₓ: 1.0, x₀: -2.0, dx₀: 3.0)"
        );
        assert_eq!(
            format!(
                "{:?}",
                crate::BoundedFrictionSimulation::new(1.0, -2.0, 3.0, -4.0, 5.0)
            ),
            "BoundedFrictionSimulation(cₓ: 1.0, x₀: -2.0, dx₀: 3.0, x: -4.0..5.0)"
        );
        assert_eq!(
            format!("{:?}", crate::SpringDescription::new(1.0, -2.0, 3.0)),
            "SpringDescription(mass: 1.0, stiffness: -2.0, damping: 3.0)"
        );
        assert_eq!(
            format!(
                "{:?}",
                crate::SpringDescription::with_damping_ratio(1.0, 9.0)
            ),
            "SpringDescription(mass: 1.0, stiffness: 9.0, damping: 6.0)"
        );
        assert_eq!(
            format!(
                "{:?}",
                crate::SpringSimulation::new(
                    crate::SpringDescription::new(1.0, 2.0, 3.0),
                    0.0,
                    1.0,
                    2.0,
                )
            ),
            "SpringSimulation(end: 1.0, SpringType.overDamped)"
        );
    }
}
