# reveal-animation/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/animation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Handles

- Change: the controller, the concrete animations and the tweens are arena objects whose methods take `self: Handle<Self>` plus the App, per the receiver rule in `reveal-foundation/src/PORTING.md` (app.rs); the listener mixin traits and `Animation` use the same receiver.
  Reason: language — see the foundation entry.
  Affect: the handles are `Listenable`. Implement the mixin traits and `Animation` for the object type, not for the handle; a crate that calls the inherent methods needs `#![feature(arbitrary_self_types)]`.

## animation.rs → animation.dart

- Change: `AnyAnimation::value` returns an owned `T`, and the type is not foundation's `ValueListenable`.
  Reason: language — an animation may compute its value on read (`CurvedAnimation`), so there is no stored field for a `&T` to point at.
  Affect: handing an animation to a `ValueListenable` parameter waits for that site (`AnimatedBuilder`).

- Change: `AnimationStatusListener` is a handle matched by identity, like `Listener`.
  Reason: language — a Rust closure has no identity; a named function has no stable address.
  Affect: store the `AnimationStatusListener::new` closure you add, or use `handle_method` at both add and remove.

- Change: Dart's `animation is A` / `animation as A` is `AnyAnimation::downcast`, and `id` exposes the arena id behind the type-erased handle.
  Reason: language — a type-erased handle is an id plus a static vtable, so a runtime type test has to go through the arena rather than a fat pointer.
  Affect: `animation.downcast::<TrainHoppingAnimation>(app)` returns `Option<Handle<TrainHoppingAnimation>>`.

## listener_helpers.rs → listener_helpers.dart

- Change: Dart's four mixins are traits whose methods take `self: Handle<Self>`; each mixin's state is an opaque `XxxData` field on the host, and the trait keeps Flutter's `XxxMixin` name.
  Reason: language — a Rust trait holds no state, and a `&mut self` receiver cannot also produce the `&mut App` the hooks hand onward.
  Affect: hold the data field (`lazy_listener: AnimationLazyListenerData`, and the local-listener bags) and hand it out from the host type; the eager mixin has no data. When two mixins both declare `did_register_listener`, say which one applies, and call with UFCS when both traits are in scope.

- Change: a panicking listener ends the notification.
  Reason: language — Rust has no catchable exception for ordinary control flow.
  Affect: listeners after a panicking one are skipped.

## tween.rs → tween.dart

- Change: `Tween<T>` interpolates through a `TweenLerp` trait, not `+` / `-` / `*`.
  Reason: language — Rust cannot dispatch those operators on an open `T` at runtime.
  Affect: a missing `TweenLerp` impl is a compile error, not Dart's "Cannot lerp" throw. `f64` and the dart:ui value types that carry the operators implement it; `i64`, `Color` and `Rect` use their dedicated tweens, as Dart's hint says.

## animations.rs → animations.dart

- Change: `AnimationMax` and `AnimationMin` are `f64` only.
  Reason: language — Rust has no `num` spanning `i64` and `f64`.
  Affect: a non-double use is the trigger to generalize. Comparison is Dart's `max` / `min` (NaN propagates), not `f64::max`.

## animation_controller.rs → animation_controller.dart

- Change: the running simulation is `Rc<dyn Simulation>`.
  Reason: language — the tick hands `&mut App` to listeners while evaluating the simulation; a borrow of the slot cannot survive that.
  Affect: set tolerance on the simulation before passing it in.

## animation_style.rs → animation_style.dart

- Change: curve fields compare by `Rc` identity; `_LerpedCurve`'s structural `==` is not reproduced.
  Reason: language — `dyn Curve` has no `PartialEq`, and no public `Curve` overrides `==`.
  Affect: two styles produced by `AnimationStyle::lerp` compare unequal even when their lerped curves are structurally equal.

- Change: `duration` is `std::time::Duration`.
  Reason: language — Rust's std duration is unsigned.
  Affect: extrapolation below zero saturates to zero.

## curves.rs → curves.dart

- Change: each named curve (`Curves::ease_in()`) returns a clone of one thread-local `Rc`.
  Reason: language — no const canonicalization.
  Affect: compare named curves with `Rc::ptr_eq`; an inline `Cubic::new(0.42, 0.0, 1.0, 1.0)` is not the same object as `Curves::ease_in()`.

## Deferred

- `AnimationBehavior._enableAnimations` reading `SemanticsBinding.instance.disableAnimations` (the 0.05 duration scale and the 200.0 fling-velocity scale). Trigger: accessibility; do not stub the binding.
- `Animation.fromValueListenable` / `_ValueListenableDelegateAnimation`. Trigger: first consumer; it stores a `ValueListenable` as a value.
- The `Curve2D` family: `ParametricCurve<T>`, `Curve2D`, `Curve2DSample`, `CatmullRomSpline`, `CatmullRomCurve`, `generateSamples` / `findInverse`. Trigger: a source for `dart:math`'s `Random` (sample placement), or a decision to port the family without it.
- `debugLabel`, `toString`, `toStringDetails`. Trigger: App-aware `Debug`.
