//! Flutter counterpart: `physics/spring_simulation.dart`.

use std::fmt::{self, Debug};
use std::time::Duration;

use crate::{Simulation, Tolerance, near_zero};

/// Structure that describes a spring's constants.
///
/// Used to configure a [`SpringSimulation`].
#[derive(Clone, Copy)]
pub struct SpringDescription {
    /// The mass of the spring (m).
    ///
    /// The units are arbitrary, but all springs within a system should use
    /// the same mass units.
    ///
    /// The greater the mass, the larger the amplitude of oscillation,
    /// and the longer the time to return to the equilibrium position.
    pub mass: f64,

    /// The spring constant (k).
    ///
    /// The units of stiffness are M/T², where M is the mass unit used for the
    /// value of the [`mass`](SpringDescription::mass) property, and T is the
    /// time unit used for driving the [`SpringSimulation`].
    ///
    /// Stiffness defines the spring constant, which measures the strength of
    /// the spring. A stiff spring applies more force to the object that is
    /// attached for some deviation from the rest position.
    pub stiffness: f64,

    /// The damping coefficient (c).
    ///
    /// It is a pure number without physical meaning and describes the
    /// oscillation and decay of a system after being disturbed. The larger the
    /// damping, the fewer oscillations and smaller the amplitude of the elastic
    /// motion.
    ///
    /// Do not confuse the damping _coefficient_ (c) with the damping _ratio_
    /// (ζ). To create a [`SpringDescription`] with a damping ratio, use
    /// [`with_damping_ratio`](SpringDescription::with_damping_ratio).
    ///
    /// The units of the damping coefficient are M/T, where M is the mass unit
    /// used for the value of the [`mass`](SpringDescription::mass) property,
    /// and T is the time unit used for driving the [`SpringSimulation`].
    pub damping: f64,
}

impl SpringDescription {
    /// Creates a spring given the mass, stiffness, and the damping coefficient.
    ///
    /// See [`mass`](SpringDescription::mass),
    /// [`stiffness`](SpringDescription::stiffness), and
    /// [`damping`](SpringDescription::damping) for the units of the arguments.
    pub const fn new(mass: f64, stiffness: f64, damping: f64) -> SpringDescription {
        SpringDescription {
            mass,
            stiffness,
            damping,
        }
    }

    /// Creates a spring given the mass (m), stiffness (k), and damping ratio
    /// (ζ). The damping ratio describes a gradual reduction in a spring
    /// oscillation. By using the damping ratio, you can define how rapidly the
    /// oscillations decay from one bounce to the next.
    ///
    /// The damping ratio is especially useful when trying to determining the
    /// type of spring to create. A ratio of 1.0 creates a critically damped
    /// spring, > 1.0 creates an overdamped spring and < 1.0 an underdamped one.
    ///
    /// See [`mass`](SpringDescription::mass) and
    /// [`stiffness`](SpringDescription::stiffness) for the units for those
    /// arguments. The damping ratio is unitless.
    pub fn with_damping_ratio(mass: f64, stiffness: f64) -> SpringDescription {
        SpringDescription::with_damping_ratio_value(mass, stiffness, 1.0)
    }

    /// Dart's `withDampingRatio` `ratio` argument (default 1.0).
    pub fn with_damping_ratio_value(mass: f64, stiffness: f64, ratio: f64) -> SpringDescription {
        SpringDescription {
            mass,
            stiffness,
            damping: ratio * 2.0 * (mass * stiffness).sqrt(),
        }
    }

    /// Creates a [`SpringDescription`] based on a desired animation duration
    /// and bounce.
    ///
    /// This provides an intuitive way to define a spring based on its visual
    /// properties, [`duration`](SpringDescription::duration) and
    /// [`bounce`](SpringDescription::bounce).
    ///
    /// This constructor produces the same result as SwiftUI's
    /// `spring(duration:bounce:blendDuration:)` animation.
    pub fn with_duration_and_bounce(duration: Duration, bounce: f64) -> SpringDescription {
        debug_assert!(duration.as_millis() > 0, "Duration must be positive");
        let duration_in_seconds = duration.as_millis() as f64 / 1000.0;
        let mass = 1.0;
        let stiffness = (4.0 * std::f64::consts::PI * std::f64::consts::PI * mass)
            / duration_in_seconds.powi(2);
        let damping_ratio = if bounce > 0.0 {
            1.0 - bounce
        } else {
            1.0 / (bounce + 1.0)
        };
        let damping = damping_ratio * 2.0 * (mass * stiffness).sqrt();
        SpringDescription {
            mass,
            stiffness,
            damping,
        }
    }

    /// The duration parameter used in
    /// [`with_duration_and_bounce`](SpringDescription::with_duration_and_bounce).
    ///
    /// This value defines the perceptual duration of the spring, controlling
    /// its overall pace. It is approximately equal to the time it takes for
    /// the spring to settle, but for highly bouncy springs, it instead
    /// corresponds to the oscillation period.
    ///
    /// This duration does not represent the exact time for the spring to stop
    /// moving. For example, when [`bounce`](SpringDescription::bounce) is 1,
    /// the spring oscillates indefinitely, even though duration has a finite
    /// value. To determine when the motion has effectively stopped within a
    /// certain tolerance, use [`SpringSimulation::is_done`](Simulation::is_done).
    ///
    /// Defaults to 0.5 seconds.
    pub fn duration(&self) -> Duration {
        let duration_in_seconds = ((4.0 * std::f64::consts::PI * std::f64::consts::PI * self.mass)
            / self.stiffness)
            .sqrt();
        let milliseconds = (duration_in_seconds * 1000.0).round() as u64;
        Duration::from_millis(milliseconds)
    }

    /// The bounce parameter used in
    /// [`with_duration_and_bounce`](SpringDescription::with_duration_and_bounce).
    ///
    /// This value controls how bouncy the spring is:
    ///
    ///  * A value of 0 results in a critically damped spring with no oscillation.
    ///  * Values between 0 and 1 produce underdamping, where the spring oscillates a few times
    ///    before settling. A value of 1 represents an undamped spring that
    ///    oscillates indefinitely.
    ///  * Negative values indicate overdamping, where the motion is slow and
    ///    resistive, like moving through a thick fluid.
    ///
    /// Defaults to 0.
    pub fn bounce(&self) -> f64 {
        let damping_ratio = self.damping / (2.0 * (self.mass * self.stiffness).sqrt());
        if damping_ratio < 1.0 {
            1.0 - damping_ratio
        } else {
            (1.0 / damping_ratio) - 1.0
        }
    }
}

impl Debug for SpringDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpringDescription(mass: {:.1}, stiffness: {:.1}, damping: {:.1})",
            self.mass, self.stiffness, self.damping
        )
    }
}

/// The kind of spring solution that the [`SpringSimulation`] is using to
/// simulate the spring.
///
/// See [`SpringSimulation::spring_type`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpringType {
    /// A spring that does not bounce and returns to its rest position in the
    /// shortest possible time.
    CriticallyDamped,
    /// A spring that bounces.
    UnderDamped,
    /// A spring that does not bounce but takes longer to return to its rest
    /// position than a [`CriticallyDamped`](SpringType::CriticallyDamped) one.
    OverDamped,
}

impl fmt::Display for SpringType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Debug for SpringType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpringType::CriticallyDamped => write!(f, "SpringType.criticallyDamped"),
            SpringType::UnderDamped => write!(f, "SpringType.underDamped"),
            SpringType::OverDamped => write!(f, "SpringType.overDamped"),
        }
    }
}

/// A spring simulation.
///
/// Models a particle attached to a spring that follows Hooke's law.
pub struct SpringSimulation {
    end_position: f64,
    solution: SpringSolution,
    snap_to_end: bool,
    /// How close to the actual end of the simulation a value at a particular
    /// time must be before [`is_done`](Simulation::is_done) considers the
    /// simulation to be "done".
    pub tolerance: Tolerance,
}

impl SpringSimulation {
    /// Creates a spring simulation from the provided spring description, start
    /// distance, end distance, and initial velocity.
    ///
    /// The units for the start and end distance arguments are arbitrary, but
    /// must be consistent with the units used for other lengths in the system.
    ///
    /// The units for the velocity are L/T, where L is the aforementioned
    /// arbitrary unit of length, and T is the time unit used for driving the
    /// [`SpringSimulation`].
    ///
    /// If `snap_to_end` is true, [`x`](Simulation::x) will be set to `end` and
    /// [`dx`](Simulation::dx) to 0 when [`is_done`](Simulation::is_done)
    /// returns true. This is useful for transitions that require the simulation
    /// to stop exactly at the end value, since the spring may not naturally
    /// reach the target precisely. Defaults to false.
    pub fn new(spring: SpringDescription, start: f64, end: f64, velocity: f64) -> SpringSimulation {
        SpringSimulation::with_options(
            spring,
            start,
            end,
            velocity,
            false,
            Tolerance::DEFAULT_TOLERANCE,
        )
    }

    /// Dart's `snapToEnd` / `tolerance` named arguments.
    pub fn with_options(
        spring: SpringDescription,
        start: f64,
        end: f64,
        velocity: f64,
        snap_to_end: bool,
        tolerance: Tolerance,
    ) -> SpringSimulation {
        SpringSimulation {
            end_position: end,
            solution: SpringSolution::new(spring, start - end, velocity),
            snap_to_end,
            tolerance,
        }
    }

    /// The kind of spring being simulated, for debugging purposes.
    ///
    /// This is derived from the [`SpringDescription`] provided to the
    /// [`SpringSimulation::new`] constructor.
    pub fn spring_type(&self) -> SpringType {
        self.solution.spring_type()
    }
}

impl Simulation for SpringSimulation {
    fn x(&self, time: f64) -> f64 {
        if self.snap_to_end && self.is_done(time) {
            self.end_position
        } else {
            self.end_position + self.solution.x(time)
        }
    }

    fn dx(&self, time: f64) -> f64 {
        if self.snap_to_end && self.is_done(time) {
            0.0
        } else {
            self.solution.dx(time)
        }
    }

    fn is_done(&self, time: f64) -> bool {
        near_zero(self.solution.x(time), self.tolerance.distance)
            && near_zero(self.solution.dx(time), self.tolerance.velocity)
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

impl Debug for SpringSimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpringSimulation(end: {:.1}, {})",
            self.end_position,
            self.spring_type()
        )
    }
}

/// A [`SpringSimulation`] where the value of [`x`](Simulation::x) is
/// guaranteed to have exactly the end value when the simulation
/// [`is_done`](Simulation::is_done).
pub struct ScrollSpringSimulation {
    inner: SpringSimulation,
}

impl ScrollSpringSimulation {
    /// Creates a spring simulation from the provided spring description, start
    /// distance, end distance, and initial velocity.
    ///
    /// See [`SpringSimulation::new`] on the superclass for a discussion of the
    /// arguments' units.
    pub fn new(
        spring: SpringDescription,
        start: f64,
        end: f64,
        velocity: f64,
        tolerance: Tolerance,
    ) -> ScrollSpringSimulation {
        ScrollSpringSimulation {
            inner: SpringSimulation::with_options(spring, start, end, velocity, false, tolerance),
        }
    }
}

impl Simulation for ScrollSpringSimulation {
    fn x(&self, time: f64) -> f64 {
        if self.is_done(time) {
            self.inner.end_position
        } else {
            self.inner.x(time)
        }
    }

    fn dx(&self, time: f64) -> f64 {
        self.inner.dx(time)
    }

    fn is_done(&self, time: f64) -> bool {
        self.inner.is_done(time)
    }

    fn tolerance(&self) -> Tolerance {
        self.inner.tolerance()
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.inner.set_tolerance(tolerance);
    }
}

impl Debug for ScrollSpringSimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

enum SpringSolution {
    Critical { r: f64, c1: f64, c2: f64 },
    Overdamped { r1: f64, r2: f64, c1: f64, c2: f64 },
    Underdamped { w: f64, r: f64, c1: f64, c2: f64 },
}

impl SpringSolution {
    fn new(
        spring: SpringDescription,
        initial_position: f64,
        initial_velocity: f64,
    ) -> SpringSolution {
        let cmk = spring.damping * spring.damping - 4.0 * spring.mass * spring.stiffness;
        if cmk > 0.0 {
            SpringSolution::overdamped(spring, initial_position, initial_velocity)
        } else if cmk < 0.0 {
            SpringSolution::underdamped(spring, initial_position, initial_velocity)
        } else {
            SpringSolution::critical(spring, initial_position, initial_velocity)
        }
    }

    fn critical(spring: SpringDescription, distance: f64, velocity: f64) -> SpringSolution {
        let r = -spring.damping / (2.0 * spring.mass);
        let c1 = distance;
        let c2 = velocity - (r * distance);
        SpringSolution::Critical { r, c1, c2 }
    }

    fn overdamped(spring: SpringDescription, distance: f64, velocity: f64) -> SpringSolution {
        let cmk = spring.damping * spring.damping - 4.0 * spring.mass * spring.stiffness;
        let r1 = (-spring.damping - cmk.sqrt()) / (2.0 * spring.mass);
        let r2 = (-spring.damping + cmk.sqrt()) / (2.0 * spring.mass);
        let c2 = (velocity - r1 * distance) / (r2 - r1);
        let c1 = distance - c2;
        SpringSolution::Overdamped { r1, r2, c1, c2 }
    }

    fn underdamped(spring: SpringDescription, distance: f64, velocity: f64) -> SpringSolution {
        let w = (4.0 * spring.mass * spring.stiffness - spring.damping * spring.damping).sqrt()
            / (2.0 * spring.mass);
        let r = -(spring.damping / 2.0 / spring.mass);
        let c1 = distance;
        let c2 = (velocity - r * distance) / w;
        SpringSolution::Underdamped { w, r, c1, c2 }
    }

    fn x(&self, time: f64) -> f64 {
        match *self {
            SpringSolution::Critical { r, c1, c2 } => (c1 + c2 * time) * (r * time).exp(),
            SpringSolution::Overdamped { r1, r2, c1, c2 } => {
                c1 * (r1 * time).exp() + c2 * (r2 * time).exp()
            }
            SpringSolution::Underdamped { w, r, c1, c2 } => {
                (r * time).exp() * (c1 * (w * time).cos() + c2 * (w * time).sin())
            }
        }
    }

    fn dx(&self, time: f64) -> f64 {
        match *self {
            SpringSolution::Critical { r, c1, c2 } => {
                let power = (r * time).exp();
                r * (c1 + c2 * time) * power + c2 * power
            }
            SpringSolution::Overdamped { r1, r2, c1, c2 } => {
                c1 * r1 * (r1 * time).exp() + c2 * r2 * (r2 * time).exp()
            }
            SpringSolution::Underdamped { w, r, c1, c2 } => {
                let power = (r * time).exp();
                let cosine = (w * time).cos();
                let sine = (w * time).sin();
                power * (c2 * w * cosine - c1 * w * sine) + r * power * (c2 * sine + c1 * cosine)
            }
        }
    }

    fn spring_type(&self) -> SpringType {
        match self {
            SpringSolution::Critical { .. } => SpringType::CriticallyDamped,
            SpringSolution::Overdamped { .. } => SpringType::OverDamped,
            SpringSolution::Underdamped { .. } => SpringType::UnderDamped,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::Simulation;

    fn approx(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "{actual} != {expected} ±{epsilon}"
        );
    }

    #[test]
    fn when_snap_to_end_is_set_value_is_exactly_end_after_completion() {
        let description = SpringDescription::with_damping_ratio(1.0, 400.0);
        let time = 0.4;

        let regular = SpringSimulation::with_options(
            description,
            0.0,
            1.0,
            0.0,
            false,
            Tolerance {
                distance: 0.1,
                velocity: 0.1,
                ..Tolerance::DEFAULT_TOLERANCE
            },
        );
        assert!(regular.x(time) < 1.0);
        assert!(regular.dx(time) > 0.0);

        let snapping = SpringSimulation::with_options(
            description,
            0.0,
            1.0,
            0.0,
            true,
            Tolerance {
                distance: 0.1,
                velocity: 0.1,
                ..Tolerance::DEFAULT_TOLERANCE
            },
        );
        assert_eq!(snapping.x(time), 1.0);
        assert_eq!(snapping.dx(time), 0.0);
    }

    #[test]
    fn spring_simulation_results_are_continuous_near_critical_damping() {
        let time = 0.4;
        let stiffness = 0.4;
        let mass = 0.4;
        let critical = SpringSimulation::new(
            SpringDescription::with_damping_ratio(mass, stiffness),
            0.0,
            1.0,
            0.0,
        );
        approx(critical.x(time), 0.06155, 0.01);
        approx(critical.dx(time), 0.2681, 0.01);

        let slightly_over = SpringSimulation::new(
            SpringDescription::with_damping_ratio_value(mass, stiffness, 1.0 + 1e-3),
            0.0,
            1.0,
            0.0,
        );
        approx(slightly_over.x(time), 0.06155, 0.01);
        approx(slightly_over.dx(time), 0.2681, 0.01);

        let slightly_under = SpringSimulation::new(
            SpringDescription::with_damping_ratio_value(mass, stiffness, 1.0 - 1e-3),
            0.0,
            1.0,
            0.0,
        );
        approx(slightly_under.x(time), 0.06155, 0.01);
        approx(slightly_under.dx(time), 0.2681, 0.01);
    }

    #[test]
    fn with_duration_and_bounce_creates_spring_with_expected_results() {
        let spring = SpringDescription::with_duration_and_bounce(Duration::from_millis(500), 0.3);

        assert_eq!(spring.mass, 1.0);
        approx(spring.stiffness, 157.91, 0.01);
        approx(spring.damping, 17.59, 0.01);
        approx(spring.bounce(), 0.3, 0.0001);
        assert_eq!(spring.duration().as_millis(), 500);
    }

    #[test]
    fn with_duration_and_bounce_creates_spring_with_negative_bounce() {
        let spring = SpringDescription::with_duration_and_bounce(Duration::from_millis(500), -0.3);

        assert_eq!(spring.mass, 1.0);
        approx(spring.stiffness, 157.91, 0.01);
        approx(spring.damping, 35.90, 0.01);
        approx(spring.bounce(), -0.3, 0.0001);
        assert_eq!(spring.duration().as_millis(), 500);
    }

    #[test]
    fn get_duration_and_bounce_based_on_mass_and_stiffness() {
        let spring = SpringDescription::new(1.0, 157.91, 17.59);
        approx(spring.bounce(), 0.3, 0.001);
        assert_eq!(spring.duration().as_millis(), 500);
    }

    #[test]
    fn custom_duration() {
        let spring = SpringDescription::with_duration_and_bounce(Duration::from_millis(100), 0.0);

        assert_eq!(spring.mass, 1.0);
        approx(spring.stiffness, 3947.84, 0.01);
        approx(spring.damping, 125.66, 0.01);
        approx(spring.bounce(), 0.0, 0.001);
        assert_eq!(spring.duration().as_millis(), 100);
    }

    #[test]
    #[should_panic(expected = "Duration must be positive")]
    fn duration_zero_should_fail() {
        SpringDescription::with_duration_and_bounce(Duration::ZERO, 0.3);
    }

    #[test]
    fn spring_types() {
        let crit = SpringSimulation::new(
            SpringDescription::with_damping_ratio(1.0, 100.0),
            0.0,
            300.0,
            0.0,
        );
        assert_eq!(crit.spring_type(), SpringType::CriticallyDamped);

        let under = SpringSimulation::new(
            SpringDescription::with_damping_ratio_value(1.0, 100.0, 0.75),
            0.0,
            300.0,
            0.0,
        );
        assert_eq!(under.spring_type(), SpringType::UnderDamped);

        let over = SpringSimulation::new(
            SpringDescription::with_damping_ratio_value(1.0, 100.0, 1.25),
            0.0,
            300.0,
            0.0,
        );
        assert_eq!(over.spring_type(), SpringType::OverDamped);

        let other =
            SpringSimulation::new(SpringDescription::new(1.0, 100.0, 20.0), 0.0, 20.0, 20.0);
        assert_eq!(other.spring_type(), SpringType::CriticallyDamped);
    }

    #[test]
    fn crit_spring() {
        let mut crit = SpringSimulation::new(
            SpringDescription::with_damping_ratio(1.0, 100.0),
            0.0,
            500.0,
            0.0,
        );
        crit.tolerance = Tolerance {
            distance: 0.01,
            velocity: 0.01,
            ..Tolerance::DEFAULT_TOLERANCE
        };

        assert_eq!(crit.spring_type(), SpringType::CriticallyDamped);
        assert!(!crit.is_done(0.0));
        assert_eq!(crit.x(0.0), 0.0);
        assert_eq!(crit.dx(0.0), 0.0);

        assert_eq!(crit.x(0.25).floor() as i32, 356);
        assert_eq!(crit.x(0.50).floor() as i32, 479);
        assert_eq!(crit.x(0.75).floor() as i32, 497);

        assert_eq!(crit.dx(0.25).floor() as i32, 1026);
        assert_eq!(crit.dx(0.50).floor() as i32, 168);
        assert_eq!(crit.dx(0.75).floor() as i32, 20);

        assert!(crit.x(1.5) > 499.0 && crit.x(1.5) < 501.0);
        assert!(crit.dx(1.5) < 0.1);
        assert!(crit.is_done(1.60));
    }

    #[test]
    fn overdamped_spring() {
        let mut over = SpringSimulation::new(
            SpringDescription::with_damping_ratio_value(1.0, 100.0, 1.25),
            0.0,
            500.0,
            0.0,
        );
        over.tolerance = Tolerance {
            distance: 0.01,
            velocity: 0.01,
            ..Tolerance::DEFAULT_TOLERANCE
        };

        assert_eq!(over.spring_type(), SpringType::OverDamped);
        assert!(!over.is_done(0.0));
        assert_eq!(over.x(0.0), 0.0);
        approx(over.dx(0.0), 0.0, 1e-10);

        assert_eq!(over.x(0.5).floor() as i32, 445);
        assert_eq!(over.x(1.0).floor() as i32, 495);
        assert_eq!(over.x(1.5).floor() as i32, 499);

        assert_eq!(over.dx(0.5).floor() as i32, 273);
        assert_eq!(over.dx(1.0).floor() as i32, 22);
        assert_eq!(over.dx(1.5).floor() as i32, 1);

        assert!(over.is_done(3.0));
    }

    #[test]
    fn underdamped_spring() {
        let under = SpringSimulation::new(
            SpringDescription::with_damping_ratio_value(1.0, 100.0, 0.25),
            0.0,
            300.0,
            0.0,
        );
        assert_eq!(under.spring_type(), SpringType::UnderDamped);
        assert!(!under.is_done(0.0));
        approx(under.x(0.0), 0.0, 1e-10);
        approx(under.dx(0.0), 0.0, 1e-10);

        assert_eq!(under.x(1.0).floor() as i32, 325);
        assert_eq!(under.dx(1.0).floor() as i32, -65);

        assert_eq!(under.dx(6.0).floor() as i32, 0);
        assert_eq!(under.x(6.0).floor() as i32, 299);

        assert!(under.is_done(6.0));
    }
}
