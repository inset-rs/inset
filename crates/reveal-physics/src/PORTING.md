# reveal-physics/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/physics
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- utils.rs → utils.dart
- gravity_simulation.rs → gravity_simulation.dart

## simulation.rs → simulation.dart

- Change: Dart's abstract class `Simulation` with a mutable `tolerance` field is the trait [`Simulation`] plus `tolerance()` / `set_tolerance()`. Concrete types still expose `pub tolerance`.
  Reason: language — a Rust trait cannot hold a field.
  Affect: through a trait object, write `simulation.set_tolerance(t)` where Dart assigns `simulation.tolerance = t`. On a concrete type, `sim.tolerance = t` still works.

## spring_simulation.rs → spring_simulation.dart

- Change: Dart `Duration(seconds: -1)` cannot be constructed; only [`Duration::ZERO`] hits the "must be positive" assert.
  Reason: language — `std::time::Duration` is unsigned.
  Affect: there is no negative-duration call.

## clamped_simulation.rs → clamped_simulation.dart

- Change: Dart's `Simulation` field is `Box<dyn Simulation>`.
  Reason: language — an abstract instance is a trait object.
  Affect: wrap the inner simulation in `Box::new`.

## Deferred

- `BouncingScrollSimulation` tests in `newton_test.dart`. Trigger: widgets scroll physics.
