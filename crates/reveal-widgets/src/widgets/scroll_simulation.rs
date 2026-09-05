//! Flutter counterpart: `widgets/scroll_simulation.dart`.

use std::fmt::{self, Debug};

use reveal_embedder::clamp_double;
use reveal_physics::{
    FrictionSimulation, ScrollSpringSimulation, Simulation, SpringDescription, Tolerance,
};

/// An implementation of scroll physics that matches iOS.
///
/// See also:
///
///  * [`ClampingScrollSimulation`], which implements Android scroll physics.
pub struct BouncingScrollSimulation {
    /// When [`x`](Simulation::x) falls below this value the simulation switches from an
    /// internal friction model to a spring model which causes
    /// [`x`](Simulation::x) to "spring" back to [`leading_extent`](Self::leading_extent).
    pub leading_extent: f64,

    /// When [`x`](Simulation::x) exceeds this value the simulation switches from an
    /// internal friction model to a spring model which causes
    /// [`x`](Simulation::x) to "spring" back to [`trailing_extent`](Self::trailing_extent).
    pub trailing_extent: f64,

    /// The spring used to return [`x`](Simulation::x) to either
    /// [`leading_extent`](Self::leading_extent) or
    /// [`trailing_extent`](Self::trailing_extent).
    pub spring: SpringDescription,

    friction_simulation: Option<FrictionSimulation>,
    spring_simulation: Option<ScrollSpringSimulation>,
    spring_time: f64,

    /// How close to the actual end of the simulation a value at a particular
    /// time must be before [`is_done`](Simulation::is_done) considers the
    /// simulation to be "done".
    pub tolerance: Tolerance,
}

impl BouncingScrollSimulation {
    /// Creates a simulation group for scrolling on iOS, with the given
    /// parameters.
    ///
    /// The position and velocity arguments must use the same units as will be
    /// expected from the [`x`](Simulation::x) and [`dx`](Simulation::dx) methods
    /// respectively (typically logical pixels and logical pixels per second
    /// respectively).
    ///
    /// The leading and trailing extents must use the unit of length, the same
    /// unit as used for the position argument and as expected from the
    /// [`x`](Simulation::x) method (typically logical pixels).
    ///
    /// The units used with the provided [`SpringDescription`] must similarly be
    /// consistent with the other arguments.
    pub fn new(
        position: f64,
        velocity: f64,
        leading_extent: f64,
        trailing_extent: f64,
        spring: SpringDescription,
    ) -> BouncingScrollSimulation {
        BouncingScrollSimulation::with_options(
            position,
            velocity,
            leading_extent,
            trailing_extent,
            spring,
            0.0,
            Tolerance::DEFAULT_TOLERANCE,
        )
    }

    /// Dart's `constantDeceleration` / `tolerance` named arguments.
    pub fn with_options(
        position: f64,
        velocity: f64,
        leading_extent: f64,
        trailing_extent: f64,
        spring: SpringDescription,
        constant_deceleration: f64,
        tolerance: Tolerance,
    ) -> BouncingScrollSimulation {
        debug_assert!(leading_extent <= trailing_extent);
        let mut simulation = BouncingScrollSimulation {
            leading_extent,
            trailing_extent,
            spring,
            friction_simulation: None,
            spring_simulation: None,
            spring_time: 0.0,
            tolerance,
        };
        if position < leading_extent {
            simulation.spring_simulation =
                Some(simulation.underscroll_simulation(position, velocity));
            simulation.spring_time = f64::NEG_INFINITY;
        } else if position > trailing_extent {
            simulation.spring_simulation =
                Some(simulation.overscroll_simulation(position, velocity));
            simulation.spring_time = f64::NEG_INFINITY;
        } else {
            // Taken from UIScrollView.decelerationRate (.normal = 0.998)
            // 0.998^1000 = ~0.135
            let friction_simulation = FrictionSimulation::with_options(
                0.135,
                position,
                velocity,
                tolerance,
                constant_deceleration,
            );
            let final_x = friction_simulation.final_x();
            if velocity > 0.0 && final_x > trailing_extent {
                simulation.spring_time = friction_simulation.time_at_x(trailing_extent);
                simulation.spring_simulation = Some(
                    simulation.overscroll_simulation(
                        trailing_extent,
                        friction_simulation
                            .dx(simulation.spring_time)
                            .min(BouncingScrollSimulation::MAX_SPRING_TRANSFER_VELOCITY),
                    ),
                );
                debug_assert!(simulation.spring_time.is_finite());
            } else if velocity < 0.0 && final_x < leading_extent {
                simulation.spring_time = friction_simulation.time_at_x(leading_extent);
                simulation.spring_simulation = Some(
                    simulation.underscroll_simulation(
                        leading_extent,
                        friction_simulation
                            .dx(simulation.spring_time)
                            .min(BouncingScrollSimulation::MAX_SPRING_TRANSFER_VELOCITY),
                    ),
                );
                debug_assert!(simulation.spring_time.is_finite());
            } else {
                simulation.spring_time = f64::INFINITY;
            }
            simulation.friction_simulation = Some(friction_simulation);
        }
        simulation
    }

    /// The maximum velocity that can be transferred from the inertia of a ballistic
    /// scroll into overscroll.
    pub const MAX_SPRING_TRANSFER_VELOCITY: f64 = 5000.0;

    fn underscroll_simulation(&self, x: f64, dx: f64) -> ScrollSpringSimulation {
        ScrollSpringSimulation::new(self.spring, x, self.leading_extent, dx, self.tolerance)
    }

    fn overscroll_simulation(&self, x: f64, dx: f64) -> ScrollSpringSimulation {
        ScrollSpringSimulation::new(self.spring, x, self.trailing_extent, dx, self.tolerance)
    }

    /// Dart's `_simulation`, which also stores the time offset in `_timeOffset` for the
    /// caller to subtract; that scratch field is this second return value.
    fn simulation(&self, time: f64) -> (&dyn Simulation, f64) {
        if time > self.spring_time {
            let time_offset = if self.spring_time.is_finite() {
                self.spring_time
            } else {
                0.0
            };
            let spring = self
                .spring_simulation
                .as_ref()
                .expect("a spring time below infinity means a spring simulation was built");
            (spring, time_offset)
        } else {
            let friction = self
                .friction_simulation
                .as_ref()
                .expect("a finite spring time means the friction simulation was built");
            (friction, 0.0)
        }
    }
}

impl Simulation for BouncingScrollSimulation {
    fn x(&self, time: f64) -> f64 {
        let (simulation, time_offset) = self.simulation(time);
        simulation.x(time - time_offset)
    }

    fn dx(&self, time: f64) -> f64 {
        let (simulation, time_offset) = self.simulation(time);
        simulation.dx(time - time_offset)
    }

    fn is_done(&self, time: f64) -> bool {
        let (simulation, time_offset) = self.simulation(time);
        simulation.is_done(time - time_offset)
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    /// Dart's `_simulation` assigns this simulation's tolerance to whichever inner
    /// simulation it hands back, on every query; the inner simulations are built with it
    /// and it is pushed down here instead, which no caller can tell apart.
    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
        if let Some(friction) = &mut self.friction_simulation {
            friction.set_tolerance(tolerance);
        }
        if let Some(spring) = &mut self.spring_simulation {
            spring.set_tolerance(tolerance);
        }
    }
}

impl Debug for BouncingScrollSimulation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BouncingScrollSimulation(leadingExtent: {:?}, trailingExtent: {:?})",
            self.leading_extent, self.trailing_extent
        )
    }
}

/// An implementation of scroll physics that aligns with Android.
///
/// For any value of [`velocity`](Self::velocity), this travels the same total distance as
/// the Android scroll physics.
///
/// This scroll physics has been adjusted relative to Android's in order to make
/// it ballistic, meaning that the deceleration at any moment is a function only
/// of the current velocity [`dx`](Simulation::dx) and does not depend on how long ago the
/// simulation was started. (This is required by Flutter's scrolling protocol,
/// where `ScrollActivityDelegate.goBallistic` may restart a scroll activity
/// using only its current velocity and the scroll position's own state.)
/// Compared to this scroll physics, Android's moves faster at the very
/// beginning, then slower, and it ends at the same place but a little later.
///
/// Times are measured in seconds, and positions in logical pixels.
///
/// See also:
///
///  * [`BouncingScrollSimulation`], which implements iOS scroll physics.
//
// This class is based on OverScroller.java from Android:
//   https://android.googlesource.com/platform/frameworks/base/+/android-13.0.0_r24/core/java/android/widget/OverScroller.java#738
// and in particular class SplineOverScroller (at the end of the file), starting
// at method "fling".  (A very similar algorithm is in Scroller.java in the same
// directory, but OverScroller is what's used by RecyclerView.)
//
// In the Android implementation, times are in milliseconds, positions are in
// physical pixels, but velocity is in physical pixels per whole second.
//
// The "See..." comments below refer to SplineOverScroller methods and values.
#[derive(Debug)]
pub struct ClampingScrollSimulation {
    /// The position of the particle at the beginning of the simulation, in
    /// logical pixels.
    pub position: f64,

    /// The velocity at which the particle is traveling at the beginning of the
    /// simulation, in logical pixels per second.
    pub velocity: f64,

    /// The amount of friction the particle experiences as it travels.
    ///
    /// The more friction the particle experiences, the sooner it stops and the
    /// less far it travels.
    ///
    /// The default value causes the particle to travel the same total distance
    /// as in the Android scroll physics.
    // See mFlingFriction.
    pub friction: f64,

    /// The total time the simulation will run, in seconds.
    duration: f64,

    /// The total, signed, distance the simulation will travel, in logical pixels.
    distance: f64,

    /// How close to the actual end of the simulation a value at a particular
    /// time must be before [`is_done`](Simulation::is_done) considers the
    /// simulation to be "done".
    pub tolerance: Tolerance,
}

// See DECELERATION_RATE. A `static final` in Dart; `f64::ln` is not a `const fn`.
fn k_deceleration_rate() -> f64 {
    0.78_f64.ln() / 0.9_f64.ln()
}

// See INFLEXION.
const K_INFLEXION: f64 = 0.35;

// See mPhysicalCoeff.  This has a value of 0.84 times Earth gravity,
// expressed in units of logical pixels per second^2.
const PHYSICAL_COEFF: f64 = 9.80665 // g, in meters per second^2
    * 39.37 // 1 meter / 1 inch
    * 160.0 // 1 inch / 1 logical pixel
    * 0.84; // "look and feel tuning"

impl ClampingScrollSimulation {
    /// Dart's default `friction`, which causes the particle to travel the same total
    /// distance as in the Android scroll physics.
    pub const DEFAULT_FRICTION: f64 = 0.015;

    /// Creates a scroll physics simulation that aligns with Android scrolling.
    pub fn new(position: f64, velocity: f64) -> ClampingScrollSimulation {
        ClampingScrollSimulation::with_options(
            position,
            velocity,
            ClampingScrollSimulation::DEFAULT_FRICTION,
            Tolerance::DEFAULT_TOLERANCE,
        )
    }

    /// Dart's `friction` / `tolerance` named arguments.
    pub fn with_options(
        position: f64,
        velocity: f64,
        friction: f64,
        tolerance: Tolerance,
    ) -> ClampingScrollSimulation {
        let mut simulation = ClampingScrollSimulation {
            position,
            velocity,
            friction,
            duration: 0.0,
            distance: 0.0,
            tolerance,
        };
        simulation.duration = simulation.fling_duration();
        simulation.distance = simulation.fling_distance();
        simulation
    }

    // See getSplineFlingDuration().
    fn fling_duration(&self) -> f64 {
        // See getSplineDeceleration().  That function's value is
        // (velocity.abs() / referenceVelocity).ln().
        let reference_velocity = self.friction * PHYSICAL_COEFF / K_INFLEXION;

        // This is the value getSplineFlingDuration() would return, but in seconds.
        let android_duration =
            (self.velocity.abs() / reference_velocity).powf(1.0 / (k_deceleration_rate() - 1.0));

        // We finish a bit sooner than Android, in order to travel the
        // same total distance.
        k_deceleration_rate() * K_INFLEXION * android_duration
    }

    // See getSplineFlingDistance().  This returns the same value but with the
    // sign of [`velocity`](Self::velocity), and in logical pixels.
    fn fling_distance(&self) -> f64 {
        let distance = self.velocity * self.duration / k_deceleration_rate();
        if cfg!(debug_assertions) {
            // This is the more complicated calculation that getSplineFlingDistance()
            // actually performs, which boils down to the much simpler formula above.
            let reference_velocity = self.friction * PHYSICAL_COEFF / K_INFLEXION;
            let log_velocity = (self.velocity.abs() / reference_velocity).ln();
            let distance_again = self.friction
                * PHYSICAL_COEFF
                * (log_velocity * k_deceleration_rate() / (k_deceleration_rate() - 1.0)).exp();
            debug_assert!((distance.abs() - distance_again).abs() < self.tolerance.distance);
        }
        distance
    }
}

impl Simulation for ClampingScrollSimulation {
    fn x(&self, time: f64) -> f64 {
        let t = clamp_double(time / self.duration, 0.0, 1.0);
        self.position + self.distance * (1.0 - (1.0 - t).powf(k_deceleration_rate()))
    }

    fn dx(&self, time: f64) -> f64 {
        let t = clamp_double(time / self.duration, 0.0, 1.0);
        self.velocity * (1.0 - t).powf(k_deceleration_rate() - 1.0)
    }

    fn is_done(&self, time: f64) -> bool {
        time >= self.duration
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_spring() -> SpringDescription {
        SpringDescription::with_damping_ratio_value(0.5, 100.0, 1.1)
    }

    /// Runs the simulation to its end at 60Hz and returns the time it stopped at.
    fn run_to_completion(simulation: &dyn Simulation, deadline: f64) -> f64 {
        let mut time = 0.0;
        while !simulation.is_done(time) {
            time += 1.0 / 60.0;
            assert!(
                time < deadline,
                "the simulation must finish before {deadline}s"
            );
        }
        time
    }

    /// `scroll_simulation_test.dart`: "ClampingScrollSimulation has a stable initial
    /// conditions".
    #[test]
    fn clamping_scroll_simulation_has_stable_initial_conditions() {
        for (position, velocity) in [
            (51.0, 2866.91537),
            (584.0, 2617.294734),
            (0.0, 1831.366634),
            (-156.2, 1541.57665),
            (5469.0, 182.114534),
        ] {
            let simulation = ClampingScrollSimulation::new(position, velocity);
            assert!((simulation.x(0.0) - position).abs() < 1e-9);
            assert!((simulation.dx(0.0) - velocity).abs() < 1e-9);
        }
    }

    /// `scroll_simulation_test.dart`: "ClampingScrollSimulation only decelerates, never
    /// speeds up".
    #[test]
    fn clamping_scroll_simulation_only_decelerates() {
        let simulation = ClampingScrollSimulation::new(0.0, 8000.0);
        let mut time = 0.0;
        let mut velocity = simulation.dx(time);
        while !simulation.is_done(time) {
            assert!(time < 3.0);
            time += 1.0 / 60.0;
            let next_velocity = simulation.dx(time);
            assert!(next_velocity <= velocity);
            velocity = next_velocity;
        }
        assert!(velocity.abs() < 1e-9);
    }

    /// The fling duration and distance Android's `SplineOverScroller` produces for a
    /// 8000 logical-pixel-per-second fling.
    #[test]
    fn clamping_scroll_simulation_travels_its_fling_distance_over_its_fling_duration() {
        let simulation = ClampingScrollSimulation::new(0.0, 8000.0);
        assert!(!simulation.is_done(2.1183617));
        assert!(simulation.is_done(2.118361712));
        assert!((simulation.x(2.2) - 7186.362753988744).abs() < 1e-6);
        assert_eq!(simulation.dx(2.2), 0.0);
    }

    /// A fling that stays inside the extents runs the friction model to a stop.
    #[test]
    fn bouncing_scroll_simulation_uses_friction_inside_the_extents() {
        let simulation =
            BouncingScrollSimulation::new(0.0, 200.0, -1000.0, 1000.0, default_spring());
        assert_eq!(simulation.x(0.0), 0.0);
        assert_eq!(simulation.dx(0.0), 200.0);
        let time = run_to_completion(&simulation, 10.0);
        let resting = simulation.x(time);
        assert!(resting > 0.0 && resting < 1000.0, "stopped at {resting}");
    }

    /// A position past the leading extent starts on the spring and settles on the extent.
    #[test]
    fn bouncing_scroll_simulation_springs_back_to_the_leading_extent() {
        let simulation = BouncingScrollSimulation::new(-100.0, 0.0, 0.0, 500.0, default_spring());
        assert_eq!(simulation.x(0.0), -100.0);
        let time = run_to_completion(&simulation, 10.0);
        assert!(
            simulation.x(time).abs() < 0.1,
            "settled at {}",
            simulation.x(time)
        );
    }

    /// A fling fast enough to leave the trailing extent hands off to the spring, which
    /// brings it back to that extent.
    #[test]
    fn bouncing_scroll_simulation_hands_a_fast_fling_to_the_trailing_spring() {
        let simulation = BouncingScrollSimulation::new(0.0, 5000.0, 0.0, 100.0, default_spring());
        // The friction phase alone would carry the particle far past the extent.
        assert!(simulation.x(0.1) > 100.0);
        let time = run_to_completion(&simulation, 10.0);
        assert!(
            (simulation.x(time) - 100.0).abs() < 0.1,
            "settled at {}",
            simulation.x(time)
        );
    }

    /// `physics/newton_test.dart`: "test_kinetic_scroll".
    #[test]
    fn bouncing_scroll_simulation_switches_from_friction_to_the_spring() {
        let spring = SpringDescription::with_damping_ratio_value(1.0, 50.0, 0.5);

        let mut scroll = BouncingScrollSimulation::new(100.0, 800.0, 0.0, 300.0, spring);
        scroll.set_tolerance(Tolerance {
            velocity: 0.5,
            distance: 0.1,
            ..Tolerance::DEFAULT_TOLERANCE
        });
        assert!(!scroll.is_done(0.0));
        assert!(!scroll.is_done(0.5)); // switch from friction to spring
        assert!(scroll.is_done(3.5));

        let mut scroll2 = BouncingScrollSimulation::new(100.0, -800.0, 0.0, 300.0, spring);
        scroll2.set_tolerance(Tolerance {
            velocity: 0.5,
            distance: 0.1,
            ..Tolerance::DEFAULT_TOLERANCE
        });
        assert!(!scroll2.is_done(0.0));
        assert!(!scroll2.is_done(0.5)); // switch from friction to spring
        assert!(scroll2.is_done(3.5));
    }

    /// `physics/newton_test.dart`: "scroll_with_inf_edge_ends".
    #[test]
    fn bouncing_scroll_simulation_stays_on_friction_with_an_infinite_trailing_extent() {
        let spring = SpringDescription::with_damping_ratio_value(1.0, 50.0, 0.5);
        let mut scroll = BouncingScrollSimulation::new(100.0, 400.0, 0.0, f64::INFINITY, spring);
        scroll.set_tolerance(Tolerance {
            velocity: 1.0,
            ..Tolerance::DEFAULT_TOLERANCE
        });

        assert!(!scroll.is_done(0.0));
        assert_eq!(scroll.x(0.0), 100.0);
        assert_eq!(scroll.dx(0.0), 400.0);

        assert!((scroll.x(1.0) - 272.0).abs() < 1.0);
        assert!((scroll.dx(1.0) - 54.0).abs() < 1.0);
        assert!((scroll.dx(2.0) - 7.0).abs() < 1.0);
        assert!(scroll.dx(3.0) < 1.0);

        assert!(scroll.is_done(5.0));
        assert!((scroll.x(5.0) - 300.0).abs() < 1.0);
    }

    /// `physics/newton_test.dart`: "over/under scroll spring".
    #[test]
    fn bouncing_scroll_simulation_overscrolls_then_springs_back() {
        let spring = SpringDescription::with_damping_ratio_value(1.0, 170.0, 1.1);
        let mut scroll = BouncingScrollSimulation::new(500.0, -7500.0, 0.0, 1000.0, spring);
        scroll.set_tolerance(Tolerance {
            velocity: 45.0,
            distance: 1.5,
            ..Tolerance::DEFAULT_TOLERANCE
        });

        assert!(!scroll.is_done(0.0));
        assert!((scroll.x(0.0) - 500.0).abs() < 1e-9);
        assert!((scroll.dx(0.0) + 7500.0).abs() < 1e-9);

        // Expect to reach 0.0 at about t=.07 at which point the simulation will
        // switch from friction to the spring
        assert!(!scroll.is_done(0.065));
        assert!((scroll.x(0.065) - 42.0).abs() < 1.0);
        assert!((scroll.dx(0.065) + 6584.0).abs() < 1.0);

        // We've overscrolled (0.1 > 0.07). Trigger the underscroll
        // simulation, and reverse direction
        assert!(!scroll.is_done(0.1));
        assert!((scroll.x(0.1) + 123.0).abs() < 1.0);
        assert!((scroll.dx(0.1) + 2613.0).abs() < 1.0);

        // Headed back towards 0.0 and slowing down.
        assert!(!scroll.is_done(0.5));
        assert!((scroll.x(0.5) + 15.0).abs() < 1.0);
        assert!((scroll.dx(0.5) - 124.0).abs() < 1.0);

        // Now jump back to the beginning, because we can.
        assert!(!scroll.is_done(0.0));
        assert!((scroll.x(0.0) - 500.0).abs() < 1e-9);
        assert!((scroll.dx(0.0) + 7500.0).abs() < 1e-9);

        assert!(scroll.is_done(2.0));
        assert_eq!(scroll.x(2.0), 0.0);
        assert!(scroll.dx(2.0).abs() < 1.0);
    }

    #[test]
    fn bouncing_scroll_simulation_to_string_names_its_extents() {
        let simulation = BouncingScrollSimulation::new(0.0, 0.0, -1.0, 500.0, default_spring());
        assert_eq!(
            format!("{simulation:?}"),
            "BouncingScrollSimulation(leadingExtent: -1.0, trailingExtent: 500.0)"
        );
    }
}
