# reveal-animation/src
Flutter home: packages/flutter/lib/src/animation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- curves.rs → curves.dart (Curve2D family deferred)

## animation.rs → animation.dart

- Change: Dart's `abstract class Animation<T>` is two types: a concrete animation implements [`AnimationNode<T>`], and a field typed `Animation<T>` holds the [`Animation<T>`] handle, built with [`Animation::from_handle`].
  Reason: language — Rust has no subtyping, so a field cannot hold "any animation" by naming a base class. Dispatch stored in the arena would keep the arena borrowed across the listener call that needs `&mut App`.
  Affect: implement [`AnimationNode<T>`] for the state type. Where Dart declares `Animation<double>`, declare `Animation<f64>`. Erasing does not consume the typed handle. `==` is slot identity.

- Change: [`Animation::value`] returns an owned `T`. The type does not implement foundation's [`ValueListenable`].
  Reason: language — an animation may compute its value on read (`CurvedAnimation`), so there is no stored field for a `&T` to point at.
  Affect: `animation.value(app)`. Handing an animation to a `ValueListenable` parameter waits for that site (`AnimatedBuilder`).

- Change: [`AnimationStatusListener`] is a handle matched by identity, like [`Listener`].
  Reason: language — a Rust closure has no identity; a named function has no stable address.
  Affect: method tear-offs use [`AnimationStatusListener::handle_method`] at both add and remove; store a [`AnimationStatusListener::new`] closure.

- Change: [`Animation::drive`] is inherent on [`Animation<f64>`] and takes `&mut App`.
  Reason: language — Dart's `drive<U>` is a generic virtual method; no dispatch table can hold a generic. Creating the driven animation needs the App.
  Affect: `controller.drive(app, tween)` where Dart writes `controller.drive(tween)`. Flutter never overrides `drive`.

## listener_helpers.rs → listener_helpers.dart

- Change: Dart's four mixins are traits on the handle. Mixin state is an opaque `XxxData` field named after the mixin; the trait is named Flutter's `XxxMixin`.
  Reason: language — a Rust trait holds no state, and a `&mut self` receiver cannot also produce the `&mut App` the hooks hand onward.
  Affect: hold `lazy_listener: AnimationLazyListenerData` (and the local-listener bags) on the host and implement `lazy_listener_data` / `lazy_listener_data_mut`. `AnimationEagerListenerMixin` has no data. When two mixins both declare `did_register_listener`, write which one applies. Call with UFCS when both traits are in scope.

- Change: a panicking listener ends the notification.
  Reason: language — Rust has no catchable exception for ordinary control flow.
  Affect: listeners after a panicking one are skipped.

## tween.rs → tween.dart

- Change: [`Tween`], [`CurveTween`], and the dedicated tween types are handle newtypes. [`Animatable::transform`] takes `&App`.
  Reason: language — Flutter mutates `tween.end` after `drive` and the driven animation must see it; a value tween cloned into `drive` would go stale. `transform` reads those fields from the arena.
  Affect: `Tween::new(app, begin, end)`, then `tween.set_end(app, Some(0.4))` where Dart assigns `tween.end = 0.4`.

- Change: [`Tween<T>`] interpolates through [`TweenLerp`], not `+` / `-` / `*`.
  Reason: language — Rust cannot dispatch those operators on an open `T` at runtime.
  Affect: a missing `TweenLerp` impl is a compile error, not Dart's "Cannot lerp" throw. `f64` is implemented; `i64` uses [`IntTween`] / [`StepTween`].

- Change: [`Animatable::animate`] and [`chain`] are default methods that require `Self: Clone`. They stay generic; [`Tween`] is `Copy`, so those call sites do not wrap in `Arc`.
  Reason: language — `drive` / `animate` are generic over one concrete animatable.
  Affect: `tween.animate(app, parent)` and `tween.chain(curve_tween)`.

- Change: [`TweenSequenceItem`] stores `Arc<dyn Animatable<T>>`.
  Reason: language — a sequence holds a mixed bag of items, so the item type is erased.
  Affect: wrap each sequence item with `Arc::new`.

## animations.rs → animations.dart

- Change: `kAlwaysCompleteAnimation` / `kAlwaysDismissedAnimation` are `k_always_complete_animation(app)` / `k_always_dismissed_animation(app)`, each the App's singleton.
  Reason: language — a handle needs a slot; `App::singleton` is the per-App counterpart of Dart's per-isolate const object.
  Affect: call the function; repeated calls are one identity.

- Change: constructors that register listeners are `create(app, …)`.
  Reason: language — a Rust value has no handle until it is in the App, and those constructors pass `this` outward.
  Affect: `CurvedAnimation::create(app, parent, curve, reverse_curve)`.

- Change: [`ProxyAnimation`] is a handle newtype. `new` takes `&mut App`. `parent =` is [`ProxyAnimation::set_parent`].
  Reason: language — methods need `&mut App`, and an inherent `impl Handle<T>` is an orphan (`Handle` is foreign).
  Affect: `ProxyAnimation::new(app, None)`, then `proxy.set_parent(app, Some(animation))`. Pass `proxy.as_animation()` where Dart passes the proxy as an `Animation<double>`.

- Change: [`AnimationMax`] and [`AnimationMin`] are `f64` only.
  Reason: language — Rust has no `num` spanning `i64` and `f64`.
  Affect: a non-double use is the trigger to generalize. Comparison is Dart's `max`/`min` (NaN propagates), not `f64::max`.

## animation_controller.rs → animation_controller.dart

- Change: the public type is the newtype [`AnimationController`], wrapping the arena slot.
  Reason: language — methods need `&mut App`, and an inherent `impl Handle<T>` is an orphan (`Handle` is foreign).
  Affect: `AnimationController::create(app, …, vsync)`, then `controller.forward(app, None)`. `controller.view()` is the erased [`Animation<f64>`].

- Change: Dart's `_directionSetter` tear-off is a cell the simulation writes and the controller drains after each `x()`.
  Reason: language — [`Simulation::x`] is `&self` with no App; the setter mutates the controller and notifies, which needs `&mut App`.
  Affect: none at the call site. Status listeners still fire before the tick assigns the new value.

- Change: the running simulation is `Rc<dyn Simulation>`.
  Reason: language — the tick hands `&mut App` to listeners while evaluating the simulation; a borrow of the slot cannot survive that.
  Affect: set tolerance on the simulation before passing it in.

- Change: Dart-optional arguments are explicit.
  Reason: language — Rust has no optional named arguments.
  Affect: `controller.stop(app, true)` for Dart's `stop()`; `Curves::linear()` where Dart omits `curve`; `None` for omitted `from` / `duration` / `min` / `max` / `period` / `count`.

## animation_style.rs → animation_style.dart

- Change: curve fields compare by `Rc` identity. `_LerpedCurve`'s structural `==` is not reproduced.
  Reason: language — `dyn Curve` has no `PartialEq`; no public `Curve` overrides `==`.
  Affect: none in the framework. Repeated `AnimationStyle::lerp` may report two structurally equal lerped curves as changed.

- Change: `duration` is `std::time::Duration`.
  Reason: language — Rust's std duration is unsigned.
  Affect: extrapolation below zero saturates to zero.

## curves.rs → curves.dart

- Change: a curve field is `Rc<dyn Curve>`. [`Curves::ease_in`] is a function returning a clone of one thread-local `Rc`.
  Reason: language — no subtyping, and no const canonicalization.
  Affect: compare interned names with `Rc::ptr_eq`. An inline `Cubic::new(0.42, 0.0, 1.0, 1.0)` is not the same object as `Curves::ease_in()`.

- Change: [`Curve::flipped`] takes `self: Rc<dyn Curve>`.
  Reason: language — `FlippedCurve` stores the receiver.
  Affect: call it on an `Rc<dyn Curve>`.

## Deferred

- `AnimationBehavior._enableAnimations` reading `SemanticsBinding.instance.disableAnimations` (the 0.05 duration scale and the 200.0 fling-velocity scale). Trigger: accessibility; do not stub the binding.
- `Animation.fromValueListenable` / `_ValueListenableDelegateAnimation`. Trigger: first consumer; it stores a `ValueListenable` as a value.
- `ParametricCurve<T>`, `Curve2D`, `Curve2DSample`, `CatmullRomSpline`, `CatmullRomCurve`, `Curve2D.generateSamples` / `findInverse`. Trigger: a source for `dart:math`'s `Random` (sample placement), or a decision to port the 2D family without it.
- `debugLabel`, `toString`, `toStringDetails`. Trigger: App-aware `Debug`.
