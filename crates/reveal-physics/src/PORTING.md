# reveal-physics/src
Flutter home: packages/flutter/lib/src/physics
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- utils.rs → utils.dart
- gravity_simulation.rs → gravity_simulation.dart

## simulation.rs → simulation.dart

- Change: Dart's abstract class `Simulation` with a mutable `tolerance` field is the trait [`Simulation`] plus `tolerance()` / `set_tolerance()`. Concrete types still expose `pub tolerance`.
  Reason: language — a Rust trait cannot hold a field.
  Affect: through a trait object, write `simulation.set_tolerance(t)` where Dart assigns `simulation.tolerance = t`. On a concrete type, `sim.tolerance = t` still works.

## tolerance.rs → tolerance.dart

- Change: Dart's `Tolerance(velocity: 1.0)` named optional constructor is a struct update from [`Tolerance::DEFAULT_TOLERANCE`].
  Reason: language — Rust has no named optional constructor arguments.
  Affect: write `Tolerance { velocity: 1.0, ..Tolerance::DEFAULT_TOLERANCE }`.

## spring_simulation.rs → spring_simulation.dart

- Change: Dart's `type` getter is [`SpringSimulation::spring_type`].
  Reason: language — `type` is a reserved word.
  Affect: write `sim.spring_type()` where Dart writes `sim.type`.

- Change: Dart's `withDampingRatio(..., {ratio = 1.0})` is [`with_damping_ratio`] (ratio 1.0) and [`with_damping_ratio_value`] when the ratio is given.
  Reason: language — Rust has no optional named arguments.
  Affect: pass the ratio through `with_damping_ratio_value`.

- Change: Dart `Duration(seconds: -1)` cannot be constructed; only [`Duration::ZERO`] hits the "must be positive" assert.
  Reason: language — `std::time::Duration` is unsigned.
  Affect: there is no negative-duration call.

## friction_simulation.rs → friction_simulation.dart

- Change: Dart's `constantDeceleration` / `tolerance` named arguments are [`FrictionSimulation::with_options`].
  Reason: language — Rust has no optional named arguments.
  Affect: the three-argument [`new`](FrictionSimulation::new) is the Dart default; extra arguments go through `with_options`.

## clamped_simulation.rs → clamped_simulation.dart

- Change: Dart's `Simulation` field is `Box<dyn Simulation>`.
  Reason: language — an abstract instance is a trait object.
  Affect: wrap the inner simulation in `Box::new`.

## Deferred

- `BouncingScrollSimulation` tests in `newton_test.dart`. Trigger: widgets scroll physics.
