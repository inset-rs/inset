# reveal-foundation/src
Flutter home: packages/flutter/lib/src/foundation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- observer_list.rs → observer_list.dart

## app.rs

- Change: `App` and `Handle<T>` live here; Flutter has no counterpart.
  Reason: language — Rust has no isolate-global GC, so a named owner has to hold the arena.
  Affect: Flutter objects are `Handle`s. Callbacks take `&mut App` plus a handle to themselves.

- Change: `Handle<T>` is a method receiver. A Flutter object is one struct in the arena, and its methods take `self: Handle<Self>` plus `&App` / `&mut App`; the body reads `app.get(self)`.
  Reason: language — `Handle` is foreign to every other crate, so `impl Handle<T>` there is an orphan; nightly `arbitrary_self_types` lets the methods live on `T` instead of a newtype around the handle.
  Affect: `let controller = AnimationController::new(app, ..)` is a `Handle<AnimationController>`; call `controller.forward(app)`. A crate that calls an inherent `self: Handle<Self>` method needs `#![feature(arbitrary_self_types)]`; trait methods resolve without it.

- Change: Dart's isolate microtask queue is on `App`. A drain has a budget and panics on a cycle.
  Reason: language — there is no isolate event loop; a Rust `Future` would need `&mut App` in `poll`. The isolate has no drain budget — a cycle hangs.
  Affect: port `scheduleMicrotask(f)` as `app.schedule_microtask(...)`. Drain after platform events and between begin-frame and draw-frame.

- Change: `dart:async` `Timer(duration, callback)` is [`Timer::new`](Timer::new). The queue lives in `timers.rs`; `App` forwards. The clock is logical: [`App::elapse`](App::elapse) fires due timers (microtasks first, then due order, microtasks after each). `Timer.periodic` is omitted.
  Reason: language — there is no isolate event loop; tests play FakeAsync via `elapse`.
  Affect: write `Timer::new(app, duration, Listener::new(...))`, `timer.cancel(app)`, `timer.is_active(app)`. Tests call `app.elapse(duration)`.

- Change: `App` holds the host `Platform`. `App::new` uses an inert one; `with_platform` installs a live one before user code.
  Reason: platform — dart:ui is host-bound; there is no isolate global to hang it on.
  Affect: host requests and view queries go through `app.platform()`.

- Change: `TargetPlatform` is re-exported from `reveal-embedder`. Flutter defines the enum in this file; `defaultTargetPlatform` is not a getter here.
  Reason: platform — the host `Platform` trait lives in the embedder crate, and that crate cannot depend on foundation.
  Affect: `app.platform().target_platform()` where Dart writes `defaultTargetPlatform`. The enum is `reveal_foundation::TargetPlatform` or `reveal_embedder::TargetPlatform`.

## key.rs → key.dart

- Change: Dart's `Key('x')` factory is `<dyn Key>::new("x")`.
  Reason: language — a trait has no constructor; an inherent on `dyn Key` is the factory without colliding with [`UniqueKey::new`](UniqueKey::new).
  Affect: write `<dyn Key>::new("x")` where Dart writes `Key('x')`. `ValueKey::new(3)` is unchanged.

- Change: [`Key`](Key) equality and hashing go through [`eq_key`](Key::eq_key) / [`hash_key`](Key::hash_key) so `dyn Key` can implement [`PartialEq`] and [`Hash`]. [`UniqueKey`](UniqueKey) identity is a monotonic id, not object identity.
  Reason: language — `PartialEq` and `Hash` are not object-safe; Rust has no implicit object identity for a value type.
  Affect: store `Box<dyn Key>` / `Arc<dyn Key>`. Two [`UniqueKey::new`](UniqueKey::new) calls are never equal. Copying a `UniqueKey` keeps the same id.

## basic_types.rs → basic_types.dart

- Change: [`ValueChanged`](ValueChanged) / [`ValueSetter`](ValueSetter) / [`ValueGetter`](ValueGetter) receive [`App`](App), as [`Listener`](Listener) does for `VoidCallback`.
  Reason: language — a Rust closure cannot capture what it mutates.
  Affect: `Rc::new(|app, value| …)` where Dart writes a `ValueChanged<T>` tear-off, not `void Function(T)`.

## constants.rs → constants.dart

- Change: `kProfileMode` is the constant `false`.
  Reason: language — Cargo profiles are invisible to the compiler. `cfg(debug_assertions)` is the only build distinction Rust exposes, and it splits two ways where Dart splits three.
  Affect: Dart `kProfileMode` branches never run.

- Change: `kIsWeb` and `kIsWasm` are always equal (`cfg!(target_family = "wasm")`).
  Reason: language — Rust has one wasm target family and no JS-compilation path, so there is no fact for Flutter's dart2js/dart2wasm distinction to rest on.
  Affect: `kIsWeb && !kIsWasm` is never true.

## change_notifier.rs → change_notifier.dart

- Change: Dart's `VoidCallback` is `Listener`, which receives `&mut App`. `notify_listeners` and `ValueNotifier::set_value` take `&mut App`.
  Reason: language — a Rust closure cannot capture what it mutates; a `&mut self` dispatch would borrow the owner for the whole loop, so a listener could not re-enter it.
  Affect: register `Listener::new(|app| …)`, not a `VoidCallback`. Notify with `this.notify_listeners(app)`, not `this.notifyListeners()`.

- Change: `remove_listener` matches a `Listener` handle by identity (`Rc::ptr_eq`, or the `(Handle, function)` pair for `handle_method`).
  Reason: language — Rust closures have no identity; Dart's `VoidCallback` compares by identity and tear-offs canonicalize.
  Affect: keep the `Listener` passed to `add_listener`, or rebuild a `handle_method` tear-off at the removal site.

- Change: Dart's mixin class `ChangeNotifier` is the trait [`ChangeNotifier`] plus a [`ChangeNotifierData`] field named `change_notifier`.
  Reason: language — a Rust trait holds no state. Methods live on [`Handle`] so the call site needs no extra import.
  Affect: `struct Foo { change_notifier: ChangeNotifierData, … }` and `impl ChangeNotifier for Foo { fn change_notifier_data(&self) -> &ChangeNotifierData { &self.change_notifier } fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData { &mut self.change_notifier } }`, then `this.notify_listeners(app)` where Dart writes `notifyListeners()`. Standalone `ChangeNotifier()` is `ChangeNotifierData::new()`. `this.add_listener(app, l)` where Dart writes `addListener(l)` — the trait lives on the handle so an erased `Animation<T>` can implement it.

- Change: a panicking listener ends the notification; remaining listeners are not called. `dispose` also clears `reentrantly_removed_listeners`.
  Reason: language — Rust has no catchable exception for ordinary control flow. A panic skips the compaction Dart's `catch` always reaches, and the next dispatch would underflow `count - reentrantly_removed_listeners`.
  Affect: a panicking listener skips every listener after it. Flutter still calls them.

- Change: [`Listenable`] lives on the handle: `Handle<T>` is `Listenable` when `T` implements [`ListenableObject`], which every [`ChangeNotifier`] does and an object with its own listener lists implements itself. Methods take `&self` and `&mut App`. [`ValueListenable::value`] takes `&App` and returns `&T`.
  Reason: language — a `&mut self` receiver on the slot cannot also produce the `&mut App` a registration must hand onward, and an erased `Animation<T>` is a handle, not a borrow of its slot. Returning `T` from `value` would force `T: Clone` on every value type.
  Affect: `this.add_listener(app, l)` and `this.value(app)` where Dart writes `addListener(l)` and `.value`. The data's inherent `add_listener` remains for code that already holds the slot. Moving the value out needs `Clone`.

## Deferred

- `Listenable.merge` / `_MergingListenable`. Trigger: first widget that holds a `Listenable` as a value (`AnimatedBuilder`).
- The form a `Listenable` takes when held as a value (stored, re-pointed, compared). Trigger: `AnimatedBuilder` / `ValueListenableBuilder`.
- `ChangeNotifier.maybeDispatchObjectCreation` / `memory_allocations`. Trigger: leak tracker / devtools.
- Diagnostics / `FlutterError` structured trees. Trigger: porting diagnostics; until then messages are `debug_assert!` strings.
- `GlobalKey` / `ObjectKey`. Trigger: `widgets/framework.dart`.
- `AsyncCallback` / `AsyncValueSetter` / `AsyncValueGetter` / `IterableFilter`. Trigger: the first async or iterable-filter call site.
