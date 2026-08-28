# reveal-foundation/src
Flutter home: packages/flutter/lib/src/foundation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- observer_list.rs → observer_list.dart

## app.rs

- Change: `App` and `Handle<T>` live here; Flutter has no counterpart.
  Reason: language — Rust has no isolate-global GC, so a named owner has to hold the arena.
  Affect: Flutter objects are `Handle`s. Callbacks take `&mut App` plus a handle to themselves.

- Change: Dart's isolate microtask queue is on `App`. A drain has a budget and panics on a cycle.
  Reason: language — there is no isolate event loop; a Rust `Future` would need `&mut App` in `poll`. The isolate has no drain budget — a cycle hangs.
  Affect: port `scheduleMicrotask(f)` as `app.schedule_microtask(...)`. Drain after platform events and between begin-frame and draw-frame.

- Change: `App` holds the host `Platform`. `App::new` uses an inert one; `with_platform` installs a live one before user code.
  Reason: platform — dart:ui is host-bound; there is no isolate global to hang it on.
  Affect: host requests and view queries go through `app.platform()`.

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

- Change: Dart's mixin class `ChangeNotifier` is the trait [`ChangeNotifier`] plus a [`ChangeNotifierState`] field named `notifier`.
  Reason: language — a Rust trait holds no state. Methods live on [`Handle`] so the call site needs no extra import.
  Affect: `struct Foo { notifier: ChangeNotifierState, … }` and `impl ChangeNotifier for Foo { fn notifier_state(&mut self) -> &mut ChangeNotifierState { &mut self.notifier } }`, then `this.notify_listeners(app)` where Dart writes `notifyListeners()`. Standalone `ChangeNotifier()` is `ChangeNotifierState::new()`. `add_listener` is still `app.get_mut(this).notifier.add_listener(l)`.

- Change: a panicking listener ends the notification; remaining listeners are not called. `dispose` also clears `reentrantly_removed_listeners`.
  Reason: language — Rust has no catchable exception for ordinary control flow. A panic skips the compaction Dart's `catch` always reaches, and the next dispatch would underflow `count - reentrantly_removed_listeners`.
  Affect: a panicking listener skips every listener after it. Flutter still calls them.

- Change: `ValueListenable::value` returns `&T`.
  Reason: language — returning `T` would force `T: Clone` on every value type.
  Affect: read with `.value()`; moving the value out needs `Clone`.

## Deferred

- `Listenable.merge` / `_MergingListenable`. Trigger: first widget that holds a `Listenable` as a value (`AnimatedBuilder`).
- The form a `Listenable` takes when held as a value (stored, re-pointed, compared). Trigger: `AnimatedBuilder` / `ValueListenableBuilder`.
- `ChangeNotifier.maybeDispatchObjectCreation` / `memory_allocations`. Trigger: leak tracker / devtools.
- Diagnostics / `FlutterError` structured trees. Trigger: porting diagnostics; until then messages are `debug_assert!` strings.
