# reveal-animation/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/animation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## animation.rs → animation.dart

- Change: `AnyAnimation::value` hands back an owned `T`, and an animation is not a foundation `ValueListenable`.
  Reason: language — an animation may compute its value on read (`CurvedAnimation`), so there is no stored field to borrow.
  Affect: an animation cannot be handed to anything that wants a `ValueListenable`.

- Change: an `AnimationStatusListener` is matched by identity, not by the closure it holds.
  Reason: language — a Rust closure has no identity and a named function has no stable address.
  Affect: removing with a freshly built closure removes nothing, and the listener keeps firing.

## listener_helpers.rs → listener_helpers.dart

- Change: a panicking listener ends the notification.
  Reason: language — Rust has no catchable exception for ordinary control flow.
  Affect: listeners after a panicking one are skipped.

## tween.rs → tween.dart

- Change: `Tween<T>` interpolates through a `TweenLerp` trait rather than `T`'s `+` / `-` / `*`.
  Reason: language — Rust cannot dispatch those operators on an open `T` at runtime.
  Affect: a type without the impl is rejected before the program runs, where Dart throws "Cannot lerp" on the first tick.

## animations.rs → animations.dart

- Change: `AnimationMax` and `AnimationMin` combine `f64` animations only.
  Reason: language — Rust has no numeric trait spanning `i64` and `f64`.
  Affect: an integer-valued max or min has nothing to instantiate.

## animation_controller.rs → animation_controller.dart

- Change: the running simulation is shared, not owned by the controller.
  Reason: language — the tick hands `&mut App` to listeners while evaluating the simulation, and a borrow of the slot cannot survive that.
  Affect: a simulation keeps the tolerance it carried when it was passed in; nothing adjusts it afterwards.

## animation_style.rs → animation_style.dart

- Change: curve fields compare by identity; `_LerpedCurve`'s structural `==` is not reproduced.
  Reason: language — `dyn Curve` has no `PartialEq`, and no public `Curve` overrides `==`.
  Affect: two styles produced by `AnimationStyle::lerp` compare unequal even when their lerped curves are structurally equal.

## curves.rs → curves.dart

- Change: each named curve (`Curves::ease_in()`) is a clone of one process-wide instance.
  Reason: language — no const canonicalization.
  Affect: an inline `Cubic::new(0.42, 0.0, 1.0, 1.0)` is a different curve from `Curves::ease_in()` wherever curves are compared.

## Deferred

- `AnimationBehavior`'s animations-disabled path (the 0.05 duration and 200.0 fling-velocity scales). Trigger: accessibility; do not stub the binding.
- `Animation.fromValueListenable`. Trigger: first consumer; it stores a `ValueListenable` as a value.
- The `Curve2D` family (`ParametricCurve`, `CatmullRomSpline`, `CatmullRomCurve`). Trigger: a source for `dart:math`'s `Random`, which sample placement needs.
- `debugLabel`, `toString`, `toStringDetails`. Trigger: App-aware `Debug`.
