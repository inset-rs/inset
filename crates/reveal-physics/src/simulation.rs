//! Flutter counterpart: `physics/simulation.dart`.

use std::fmt::Debug;

use crate::Tolerance;

/// The base class for all simulations.
///
/// A simulation models an object, in a one-dimensional space, on which
/// particular forces are being applied, and exposes:
///
///  * The object's position, [`x`](Simulation::x)
///  * The object's velocity, [`dx`](Simulation::dx)
///  * Whether the simulation is "done", [`is_done`](Simulation::is_done)
///
/// A simulation is generally "done" if the object has, to a given
/// [`tolerance`](Simulation::tolerance), come to a complete rest.
///
/// The [`x`](Simulation::x), [`dx`](Simulation::dx), and
/// [`is_done`](Simulation::is_done) functions take a time argument which
/// specifies the time for which they are to be evaluated. In principle,
/// simulations can be stateless, and thus can be queried with arbitrary times.
/// In practice, however, some simulations are not, and calling any of these
/// functions will advance the simulation to the given time.
///
/// As a general rule, therefore, a simulation should only be queried using
/// times that are equal to or greater than all times previously used for that
/// simulation.
///
/// Simulations do not specify units for distance, velocity, and time. Client
/// should establish a convention and use that convention consistently with all
/// related objects.
pub trait Simulation: Debug {
    /// The position of the object in the simulation at the given time.
    fn x(&self, time: f64) -> f64;

    /// The velocity of the object in the simulation at the given time.
    fn dx(&self, time: f64) -> f64;

    /// Whether the simulation is "done" at the given time.
    fn is_done(&self, time: f64) -> bool;

    /// How close to the actual end of the simulation a value at a particular
    /// time must be before [`is_done`](Simulation::is_done) considers the
    /// simulation to be "done".
    ///
    /// A simulation with an asymptotic curve would never technically be "done",
    /// but once the difference from the value at a particular time and the
    /// asymptote itself could not be seen, it would be pointless to continue.
    /// The tolerance defines how to determine if the difference could not be
    /// seen.
    fn tolerance(&self) -> Tolerance;

    /// Dart's `tolerance` setter.
    fn set_tolerance(&mut self, tolerance: Tolerance);
}
