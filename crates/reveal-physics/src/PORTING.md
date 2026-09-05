# reveal-physics/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/physics
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- utils.rs → utils.dart
- gravity_simulation.rs → gravity_simulation.dart
- spring_simulation.rs → spring_simulation.dart
- simulation.rs → simulation.dart
- clamped_simulation.rs → clamped_simulation.dart

## Deferred

- `SpringSimulation` / `FrictionSimulation` tests in `newton_test.dart` beyond the ones already here. Trigger: a change to a solution branch.
