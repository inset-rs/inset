# reveal-foundation/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/foundation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- observer_list.rs → observer_list.dart

## app.rs

- Change: `App` and `Handle<T>` live here; Flutter has no counterpart.
  Reason: language — Rust has no isolate-global GC, so a named owner has to hold the arena.
  Affect: Flutter objects are `Handle`s; callbacks take `&mut App` plus a handle to themselves.

- Change: `Handle<T>` is a method receiver: a Flutter object is one struct in the arena, and its methods take `self: Handle<Self>` plus `&App` / `&mut App`, reading `app.get(self)`.
  Reason: language — `Handle` is foreign to every other crate, so `impl Handle<T>` there is an orphan; nightly `arbitrary_self_types` lets the methods live on `T` instead of a newtype around the handle.
  Affect: `let controller = AnimationController::new(app, ..)` is a `Handle<AnimationController>`; call `controller.forward(app)`. A crate that calls an inherent `self: Handle<Self>` method needs `#![feature(arbitrary_self_types)]`; trait methods resolve without it.

- Change: Dart's isolate microtask queue is on `App`; a drain has a budget and panics on a cycle.
  Reason: language — there is no isolate event loop.
  Affect: port `scheduleMicrotask(f)` as `app.schedule_microtask(..)`. Drain after platform events and between begin-frame and draw-frame; a cycle panics where the isolate hangs.

- Change: `dart:async`'s `Timer` is `Timer::new` on a logical clock: `AppCell::elapse` fires due timers (microtasks first, then due order, microtasks after each, tasks polled after each). `Timer.periodic` is omitted.
  Reason: language — there is no isolate event loop; tests play FakeAsync via `elapse`.
  Affect: write `Timer::new(app, duration, Listener::new(..))`, `timer.cancel(app)`, `timer.is_active(app)`; tests call `cell.elapse(duration)`.

- Change: the `App` lives in an `AppCell`, which the shell and the tests hold. A Dart `async` body is ported with everything up to its first `await` inline — including the call whose future is awaited — and the rest as `app.spawn(async move |cx| ..)`, a continuation that reaches the `App` through `cx.update(|app| ..)` one closure at a time. Microtasks and continuations run only through `AppCell::checkpoint`, which repeats until neither queue has work, with nothing borrowed. `Completer` and its future are `dart:async`'s (`CompleterFuture::ready` is `Future.value`, `peek` tells a `SynchronousFuture` apart, `then(app, ..)` runs a callback at the checkpoint after completion, `wait_all` is `Future.wait`); a `Task` is a continuation's future, and dropping it does not cancel it.
  Reason: language — a Rust future cannot hold `&mut App` across an `await`, so a continuation borrows the cell for each step instead (gpui's `AppCell` / `AsyncApp`), and it cannot start inline because the caller holds the `App`.
  Affect: a Dart `Future<T> m() async {..}` is `fn m(..) -> Task<T>` whose prefix runs where Dart's does; a method that only hands back a future returns that future's type (`CompleterFuture<T>` or `Task<T>`; edition 2024's `impl Future` would capture the `&mut App`). The shell runs the checkpoint at the end of every platform event and `AppCell::elapse` runs it around every timer, so `app.drain_microtasks()` is for a borrowed `App` only; nothing else may run the checkpoint, and `cx.update` while the `App` is borrowed panics.

- Change: `App` holds the host `Platform`: `AppCell::new` uses an inert one and `AppCell::with_platform` installs a live one before user code.
  Reason: platform — dart:ui is host-bound; there is no isolate global to hang it on.
  Affect: host requests and view queries go through `app.platform()`.

## key.rs → key.dart

- Change: `Key` equality and hashing go through `eq_key` / `hash_key`, so `dyn Key` can be `PartialEq` and `Hash`; a `UniqueKey`'s identity is a monotonic id, not object identity.
  Reason: language — `PartialEq` and `Hash` are not object-safe, and a Rust value type has no implicit object identity.
  Affect: two `UniqueKey::new` calls are never equal; copying a `UniqueKey` keeps the same id.

## basic_types.rs → basic_types.dart

- Change: `ValueChanged` / `ValueSetter` / `ValueGetter` receive the `App`, as `Listener` does for `VoidCallback`.
  Reason: language — a Rust closure cannot capture what it mutates.
  Affect: `Rc::new(|app, value| …)` where Dart writes a `ValueChanged<T>` tear-off.

- Change: `AsyncCallback` / `AsyncValueSetter` / `AsyncValueGetter` return a `Task`, and a body with nothing to await returns `Task::ready`.
  Reason: language — a Rust future that nobody polls never runs, where a Dart `async` body runs whether or not its future is awaited; a `Task` is already queued when it is handed back, so a caller that ignores it (Dart's unawaited call) still lets it run.
  Affect: an implementor runs its prefix and returns `app.spawn(async move |cx| ..)`; a caller that awaits it does so in its own continuation.

## constants.rs → constants.dart

- Change: `kProfileMode` is the constant `false`.
  Reason: language — `cfg(debug_assertions)` is the only build distinction Rust exposes, and it splits two ways where Dart splits three.
  Affect: Dart's `kProfileMode` branches never run.

- Change: `kIsWeb` and `kIsWasm` are always equal (`cfg!(target_family = "wasm")`).
  Reason: language — Rust has one wasm target family and no JS-compilation path.
  Affect: `kIsWeb && !kIsWasm` is never true.

## date_time.rs → dart:core `DateTime`

- Change: only UTC exists: `DateTime::utc(year, month, day)` / `utc_with_time(..)`, `now()` read as UTC, and epoch constructors with no `isUtc`. Local time, `parse` and `difference` are absent (Deferred).
  Reason: platform — the standard library has no time zone database, and its duration is unsigned.
  Affect: `DateTime::utc(2026, 9, 5)`; subtract `microseconds_since_epoch()` values for a difference.

## change_notifier.rs → change_notifier.dart

- Change: Dart's `VoidCallback` is `Listener`, which receives `&mut App`; `notify_listeners` and `ValueNotifier::set_value` take `&mut App`.
  Reason: language — a Rust closure cannot capture what it mutates, and a `&mut self` dispatch would borrow the owner for the whole loop, so a listener could not re-enter it.
  Affect: register `Listener::new(|app| …)`; notify with `this.notify_listeners(app)`.

- Change: `remove_listener` matches a `Listener` by identity (`Rc::ptr_eq`, or the `(Handle, function)` pair for `handle_method`).
  Reason: language — Rust closures have no identity; Dart's `VoidCallback` compares by identity and tear-offs canonicalize.
  Affect: keep the `Listener` passed to `add_listener`, or rebuild a `handle_method` tear-off at the removal site.

- Change: `remove_listener` on a `Handle` whose arena slot is gone is a no-op.
  Reason: language — Dart allows `removeListener` on a disposed `ChangeNotifier`, whose object the collector still holds; here the slot may already be destroyed, and a stale `app.get_mut` panics.
  Affect: a state that unmounts after its listenable (a route's `TickerMode`, during a hero flight's pop) removes its listener without panicking, as in Dart; removal on a handle that was never valid is silently ignored too.

- Change: Dart's mixin class `ChangeNotifier` is the trait `ChangeNotifier` plus a `ChangeNotifierData` field, and the listener methods live on the handle: `Handle<T>` is `Listenable` whenever `T` implements `ListenableObject`, which every `ChangeNotifier` does and an object with its own listener lists does itself.
  Reason: language — a Rust trait holds no state, and a `&mut self` receiver on the slot cannot also produce the `&mut App` a registration hands onward.
  Affect: hold `change_notifier: ChangeNotifierData` and implement `ChangeNotifier` for the type by handing that field out; then `this.notify_listeners(app)` and `this.add_listener(app, l)` where Dart writes `notifyListeners()` and `addListener(l)`. A standalone `ChangeNotifier()` is `ChangeNotifierData::new()`; the data's inherent `add_listener` remains for code that already holds the slot.

- Change: a panicking listener ends the notification, and `dispose` also clears the reentrant-removal count.
  Reason: language — Rust has no catchable exception for ordinary control flow, so a panic skips the compaction Dart's `catch` always reaches.
  Affect: a panicking listener skips every listener after it; Flutter still calls them.

## Deferred

- date_time.rs: local time (`DateTime(..)`, `toLocal`, `timeZoneName` / `timeZoneOffset`), `parse` / `tryParse`, `difference`. Trigger: a host with a time zone database (the date picker showing `DateTime.now()`); a signed duration.
- The form a `Listenable` takes when held as a value (stored, re-pointed, compared). Trigger: `AnimatedBuilder` / `ValueListenableBuilder`.
- `ChangeNotifier.maybeDispatchObjectCreation` / `memory_allocations`. Trigger: leak tracker / devtools.
- Diagnostics / `FlutterError` structured trees. Trigger: porting diagnostics; until then messages are `debug_assert!` strings.
- `GlobalKey` / `ObjectKey`. Trigger: `widgets/framework.dart`.
- `IterableFilter`. Trigger: the first iterable-filter call site.
- Waking a task from another thread. Tasks are woken only from the main thread today, so a wake always lands before the next drain; a background task would need a `Send` poke into the host's event loop (winit's `EventLoopProxy`). Trigger: the first background executor or platform callback off the main thread.
