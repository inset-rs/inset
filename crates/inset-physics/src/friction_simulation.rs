//! Flutter counterpart: `physics/friction_simulation.dart`.

use std::fmt::{self, Debug};

use inset_embedder::clamp_double;

use crate::{Simulation, Tolerance};

/// Numerically determine the input value which produces output value `target`
/// for a function `f`, given its first-derivative `df`.
fn newtons_method(
    initial_guess: f64,
    target: f64,
    f: impl Fn(f64) -> f64,
    df: impl Fn(f64) -> f64,
    iterations: i32,
) -> f64 {
    let mut guess = initial_guess;
    for _ in 0..iterations {
        guess -= (f(guess) - target) / df(guess);
    }
    guess
}

/// A simulation that applies a drag to slow a particle down.
///
/// Models a particle affected by fluid drag, e.g. air resistance.
///
/// The simulation ends when the velocity of the particle drops to zero (within
/// the current velocity [`tolerance`](Simulation::tolerance)).
pub struct FrictionSimulation {
    drag: f64,
    drag_log: f64,
    x0: f64,
    v0: f64,
    constant_deceleration: f64,
    /// The time at which the simulation should be stopped.
    /// This is needed when constantDeceleration is not zero (on Desktop), when
    /// using the pure friction simulation, acceleration naturally reduces to zero
    /// and creates a stopping point.
    final_time: f64,
    /// How close to the actual end of the simulation a value at a particular
    /// time must be before [`is_done`](Simulation::is_done) considers the
    /// simulation to be "done".
    pub tolerance: Tolerance,
}

impl FrictionSimulation {
    /// Creates a [`FrictionSimulation`] with the given arguments, namely: the
    /// fluid drag coefficient _cₓ_, a unitless value; the initial position
    /// _x₀_, in the same length units as used for [`x`](Simulation::x); and the
    /// initial velocity _dx₀_, in the same velocity units as used for
    /// [`dx`](Simulation::dx).
    pub fn new(drag: f64, position: f64, velocity: f64) -> FrictionSimulation {
        FrictionSimulation::with_options(
            drag,
            position,
            velocity,
            Tolerance::DEFAULT_TOLERANCE,
            0.0,
        )
    }

    /// Dart's `tolerance` / `constantDeceleration` named arguments.
    pub fn with_options(
        drag: f64,
        position: f64,
        velocity: f64,
        tolerance: Tolerance,
        constant_deceleration: f64,
    ) -> FrictionSimulation {
        let constant_deceleration = constant_deceleration * velocity.signum();
        let mut simulation = FrictionSimulation {
            drag,
            drag_log: drag.ln(),
            x0: position,
            v0: velocity,
            constant_deceleration,
            final_time: f64::INFINITY,
            tolerance,
        };
        // Needs to be infinity for newtonsMethod call in constructor.
        simulation.final_time = newtons_method(
            0.0,
            0.0,
            |time| simulation.dx(time),
            |time| {
                (simulation.v0 * simulation.drag.powf(time) * simulation.drag_log)
                    - simulation.constant_deceleration
            },
            10,
        );
        simulation
    }

    /// Creates a new friction simulation with its fluid drag coefficient (_cₓ_)
    /// set so as to ensure that the simulation starts and ends at the specified
    /// positions and velocities.
    ///
    /// The positions must use the same units as expected from [`x`](Simulation::x),
    /// and the velocities must use the same units as expected from
    /// [`dx`](Simulation::dx).
    ///
    /// The sign of the start and end velocities must be the same, the magnitude
    /// of the start velocity must be greater than the magnitude of the end
    /// velocity, and the velocities must be in the direction appropriate for the
    /// particle to start from the start position and reach the end position.
    pub fn through(
        start_position: f64,
        end_position: f64,
        start_velocity: f64,
        end_velocity: f64,
    ) -> FrictionSimulation {
        debug_assert!(
            start_velocity == 0.0
                || end_velocity == 0.0
                || start_velocity.signum() == end_velocity.signum()
        );
        debug_assert!(start_velocity.abs() >= end_velocity.abs());
        debug_assert!((end_position - start_position).signum() == start_velocity.signum());
        FrictionSimulation::with_options(
            drag_for(start_position, end_position, start_velocity, end_velocity),
            start_position,
            start_velocity,
            Tolerance {
                velocity: end_velocity.abs(),
                ..Tolerance::DEFAULT_TOLERANCE
            },
            0.0,
        )
    }

    /// The value of [`x`](Simulation::x) at `f64::INFINITY`.
    pub fn final_x(&self) -> f64 {
        if self.constant_deceleration == 0.0 {
            self.x0 - self.v0 / self.drag_log
        } else {
            self.x(self.final_time)
        }
    }

    /// The time at which the value of `x(time)` will equal `x`.
    ///
    /// Returns `f64::INFINITY` if the simulation will never reach `x`.
    pub fn time_at_x(&self, x: f64) -> f64 {
        if x == self.x0 {
            return 0.0;
        }
        if self.v0 == 0.0
            || (if self.v0 > 0.0 {
                x < self.x0 || x > self.final_x()
            } else {
                x > self.x0 || x < self.final_x()
            })
        {
            return f64::INFINITY;
        }
        newtons_method(0.0, x, |time| self.x(time), |time| self.dx(time), 10)
    }
}

/// Return the drag value for a FrictionSimulation whose x() and dx() values
/// pass through the specified start and end position/velocity values.
///
/// Total time to reach endVelocity is just: (log(endVelocity) / log(startVelocity)) / log(_drag)
/// or (log(v1) - log(v0)) / log(D), given v = v0 * D^t per the dx() function below.
/// Solving for D given x(time) is trickier. Algebra courtesy of Wolfram Alpha:
/// x1 = x0 + (v0 * D^((log(v1) - log(v0)) / log(D))) / log(D) - v0 / log(D), find D
fn drag_for(start_position: f64, end_position: f64, start_velocity: f64, end_velocity: f64) -> f64 {
    ((start_velocity - end_velocity) / (start_position - end_position)).exp()
}

impl Simulation for FrictionSimulation {
    fn x(&self, time: f64) -> f64 {
        if time > self.final_time {
            return self.final_x();
        }
        self.x0 + self.v0 * self.drag.powf(time) / self.drag_log
            - self.v0 / self.drag_log
            - ((self.constant_deceleration / 2.0) * time * time)
    }

    fn dx(&self, time: f64) -> f64 {
        if time > self.final_time {
            return 0.0;
        }
        self.v0 * self.drag.powf(time) - self.constant_deceleration * time
    }

    fn is_done(&self, time: f64) -> bool {
        self.dx(time).abs() < self.tolerance.velocity
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

impl Debug for FrictionSimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FrictionSimulation(cₓ: {:.1}, x₀: {:.1}, dx₀: {:.1})",
            self.drag, self.x0, self.v0
        )
    }
}

/// A [`FrictionSimulation`] that clamps the modeled particle to a specific
/// range of values.
///
/// Only the position is clamped. The velocity [`dx`](Simulation::dx) will
/// continue to report unbounded simulated velocities once the particle has
/// reached the bounds.
pub struct BoundedFrictionSimulation {
    inner: FrictionSimulation,
    min_x: f64,
    max_x: f64,
}

impl BoundedFrictionSimulation {
    /// Creates a [`BoundedFrictionSimulation`] with the given arguments, namely:
    /// the fluid drag coefficient _cₓ_, a unitless value; the initial position
    /// _x₀_, in the same length units as used for [`x`](Simulation::x); the
    /// initial velocity _dx₀_, in the same velocity units as used for
    /// [`dx`](Simulation::dx), the minimum value for the position, and the
    /// maximum value for the position. The minimum and maximum values must be
    /// in the same units as the initial position, and the initial position must
    /// be within the given range.
    pub fn new(
        drag: f64,
        position: f64,
        velocity: f64,
        min_x: f64,
        max_x: f64,
    ) -> BoundedFrictionSimulation {
        debug_assert!(clamp_double(position, min_x, max_x) == position);
        BoundedFrictionSimulation {
            inner: FrictionSimulation::new(drag, position, velocity),
            min_x,
            max_x,
        }
    }
}

impl Simulation for BoundedFrictionSimulation {
    fn x(&self, time: f64) -> f64 {
        clamp_double(self.inner.x(time), self.min_x, self.max_x)
    }

    fn dx(&self, time: f64) -> f64 {
        self.inner.dx(time)
    }

    fn is_done(&self, time: f64) -> bool {
        self.inner.is_done(time)
            || (self.x(time) - self.min_x).abs() < self.tolerance().distance
            || (self.x(time) - self.max_x).abs() < self.tolerance().distance
    }

    fn tolerance(&self) -> Tolerance {
        self.inner.tolerance()
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.inner.set_tolerance(tolerance);
    }
}

impl Debug for BoundedFrictionSimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BoundedFrictionSimulation(cₓ: {:.1}, x₀: {:.1}, dx₀: {:.1}, x: {:.1}..{:.1})",
            self.inner.drag, self.inner.x0, self.inner.v0, self.min_x, self.max_x
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
    fn friction_simulation_positive_velocity() {
        let friction = FrictionSimulation::new(0.135, 100.0, 100.0);

        approx(friction.x(0.0), 100.0, 1e-10);
        approx(friction.dx(0.0), 100.0, 1e-10);

        approx(friction.x(0.1), 110.0, 1.0);
        approx(friction.x(0.5), 131.0, 1.0);
        approx(friction.x(2.0), 149.0, 1.0);

        approx(friction.final_x(), 149.0, 1.0);

        assert_eq!(friction.time_at_x(100.0), 0.0);
        approx(friction.time_at_x(friction.x(0.1)), 0.1, 1e-10);
        approx(friction.time_at_x(friction.x(0.5)), 0.5, 1e-10);
        approx(friction.time_at_x(friction.x(2.0)), 2.0, 1e-10);

        assert_eq!(friction.time_at_x(-1.0), f64::INFINITY);
        assert_eq!(friction.time_at_x(200.0), f64::INFINITY);
    }

    #[test]
    fn friction_simulation_negative_velocity() {
        let friction = FrictionSimulation::new(0.135, 100.0, -100.0);

        approx(friction.x(0.0), 100.0, 1e-10);
        approx(friction.dx(0.0), -100.0, 1e-10);

        approx(friction.x(0.1), 91.0, 1.0);
        approx(friction.x(0.5), 68.0, 1.0);
        approx(friction.x(2.0), 51.0, 1.0);

        approx(friction.final_x(), 50.0, 1.0);

        assert_eq!(friction.time_at_x(100.0), 0.0);
        approx(friction.time_at_x(friction.x(0.1)), 0.1, 1e-10);
        approx(friction.time_at_x(friction.x(0.5)), 0.5, 1e-10);
        approx(friction.time_at_x(friction.x(2.0)), 2.0, 1e-10);

        assert_eq!(friction.time_at_x(101.0), f64::INFINITY);
        assert_eq!(friction.time_at_x(40.0), f64::INFINITY);
    }

    #[test]
    fn friction_simulation_constant_deceleration() {
        let friction = FrictionSimulation::with_options(
            0.135,
            100.0,
            -100.0,
            Tolerance::DEFAULT_TOLERANCE,
            100.0,
        );

        approx(friction.x(0.0), 100.0, 1e-10);
        approx(friction.dx(0.0), -100.0, 1e-10);

        approx(friction.x(0.1), 91.0, 1.0);
        approx(friction.x(0.5), 80.0, 1.0);
        approx(friction.x(2.0), 80.0, 1.0);

        approx(friction.final_x(), 80.0, 1.0);

        assert_eq!(friction.time_at_x(100.0), 0.0);
        approx(friction.time_at_x(friction.x(0.1)), 0.1, 1e-10);
        approx(friction.time_at_x(friction.x(0.2)), 0.2, 1e-10);
        approx(friction.time_at_x(friction.x(0.3)), 0.3, 1e-10);

        assert_eq!(friction.time_at_x(101.0), f64::INFINITY);
        assert_eq!(friction.time_at_x(40.0), f64::INFINITY);
    }

    #[test]
    fn test_friction() {
        let mut friction = FrictionSimulation::new(0.3, 100.0, 400.0);
        friction.tolerance = Tolerance {
            velocity: 1.0,
            ..Tolerance::DEFAULT_TOLERANCE
        };

        assert!(!friction.is_done(0.0));
        assert_eq!(friction.x(0.0), 100.0);
        assert_eq!(friction.dx(0.0), 400.0);

        assert!(friction.x(1.0) > 330.0 && friction.x(1.0) < 335.0);

        assert_eq!(friction.dx(1.0), 120.0);
        assert_eq!(friction.dx(2.0), 36.0);
        approx(friction.dx(3.0), 10.8, 1e-10);
        assert!(friction.dx(4.0) < 3.5);

        assert!(friction.is_done(5.0));
        assert!(friction.x(5.0) > 431.0 && friction.x(5.0) < 432.0);
    }

    #[test]
    fn test_friction_through() {
        let start_position = 10.0;
        let start_velocity = 600.0;
        let f = FrictionSimulation::new(0.025, start_position, start_velocity);
        let end_position = f.x(1.0);
        let end_velocity = f.dx(1.0);
        assert!(end_position > start_position);
        assert!(end_velocity < start_velocity);

        let friction =
            FrictionSimulation::through(start_position, end_position, start_velocity, end_velocity);
        assert!(!friction.is_done(0.0));
        assert_eq!(friction.x(0.0), 10.0);
        assert_eq!(friction.dx(0.0), 600.0);

        assert!(friction.is_done(1.0 + 1e-10));
        approx(friction.x(1.0), end_position, 1e-10);
        approx(friction.dx(1.0), end_velocity, 1e-10);

        let start_position = 1000.0;
        let start_velocity = -500.0;
        let f = FrictionSimulation::new(0.025, 1000.0, -500.0);
        let end_position = f.x(1.0);
        let end_velocity = f.dx(1.0);
        assert!(end_position < start_position);
        assert!(end_velocity > start_velocity);

        let friction =
            FrictionSimulation::through(start_position, end_position, start_velocity, end_velocity);
        assert!(friction.is_done(1.0 + 1e-10));
        approx(friction.x(1.0), end_position, 1e-10);
        approx(friction.dx(1.0), end_velocity, 1e-10);
    }

    #[test]
    fn bounded_friction_simulation_control_test() {
        let mut friction = BoundedFrictionSimulation::new(0.3, 100.0, 400.0, 50.0, 150.0);
        friction.set_tolerance(Tolerance {
            velocity: 1.0,
            ..Tolerance::DEFAULT_TOLERANCE
        });

        assert!(!friction.is_done(0.0));
        assert_eq!(friction.x(0.0), 100.0);
        assert_eq!(friction.dx(0.0), 400.0);

        assert_eq!(friction.x(1.0), 150.0);

        assert!(friction.is_done(1.0));
    }
}
