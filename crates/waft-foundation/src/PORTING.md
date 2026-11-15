# waft-foundation/src
Flutter home: packages/flutter/lib/src/foundation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- observer_list.rs → observer_list.dart

## app.rs

- Change: `App` and `Handle<T>` live here; Flutter has no counterpart.
  Reason: language — Rust has no isolate-global GC, so a named owner has to hold the arena.
  Affect: Flutter objects are `Handle`s; callbacks take `&mut App` plus a handle to themselves.

- Change: a Flutter object is one struct in the arena and its methods take `self: Handle<Self>` plus the `App`.
  Reason: language — `Handle` is foreign to every other crate, so `impl Handle<T>` there is an orphan; nightly `arbitrary_self_types` puts the methods on `T` instead.
  Affect: a crate that calls an inherent handle-receiver method needs `#![feature(arbitrary_self_types)]`; trait methods resolve without it.

- Change: Dart's isolate microtask queue is on `App`, and a drain has a budget that panics on a cycle.
  Reason: language — there is no isolate event loop.
  Affect: the queue drains after platform events and between begin-frame and draw-frame, and a microtask cycle panics where the isolate would hang.

- Change: `dart:async`'s `Timer` runs on a logical clock that `AppCell::elapse` advances (microtasks first, then due order, microtasks after each); `Timer.periodic` is absent.
  Reason: language — there is no isolate event loop; tests play FakeAsync through `elapse`.
  Affect: no timer fires until the host or a test elapses the clock, and a repeating timer has to re-arm itself. The host is told when to wake the app for its earliest waiting timer, and told again whenever that changes, including when no timer is left. That is kept separate from a ready task asking to be woken, which is what Dart's isolate gets for free by owning both queues: holding the two in one field would let a task that became ready overwrite a timer that has not run yet.

- Change: a Dart `async` method is a `Task`: the part before its first `await` runs inline, the rest at the next checkpoint.
  Reason: language — a Rust future cannot hold the `App` across an `await`; it borrows the cell per step, as gpui does.
  Affect: everything after an `await` lands at a later checkpoint, and dropping a `Task` does not cancel it.

- Change: an object lives while a `Ref` keeps it (`App::create` returns the first, `Clone` makes more, `Handle::upgrade` makes one from a bare handle) and is freed at the checkpoint after the last one drops; an element, a state, a render object or a layer is kept by the tree code from `Retained::into_tree_node` to `App::release_tree_node`. `dispose` frees nothing, and the last drop runs no `dispose`. A debug build counts the arena's objects with `App::debug_object_count`, and by type with `App::debug_object_counts_by_type`, and reports Flutter's two leak kinds: an object freed without the `dispose` it expects (`App::debug_take_not_disposed`), and one disposed and still kept two checkpoints later (`App::debug_take_not_collected`).
  Reason: language — there is no collector, so a count per object and a free deferred to the checkpoint stand in for Dart's references, and a test for leaks reads the arena where Flutter's leak tracking watches the collector.
  Affect: an object nothing keeps can still be read until the next checkpoint and panics after it, naming its type, where it was created and the checkpoint that freed it; a ring of `Ref`s is never freed, where Dart's collector frees an unreachable ring; a release build has no count.

- Change: a tear-off (`Listener::handle_method`, `AnimationStatusListener::handle_method`, `FrameCallback::handle_method`, `PointerRoute::handle_method`) does not keep its receiver, and is skipped once the receiver is freed; a debug build reports the skipped receiver once, with its type and creation site.
  Reason: language — the tear-off names its receiver by a bare handle so that a listener list never keeps its listener alive, where Dart's tear-off keeps its receiver until it is removed.
  Affect: an object dropped while still listening is freed and its listener stops being called, where Dart would keep the object and keep calling it.

- Change: `App` holds the host `Platform`: `AppCell::new` uses an inert one and `AppCell::with_platform` installs a live one before user code.
  Reason: platform — dart:ui is host-bound; there is no isolate global to hang it on.
  Affect: host requests and view queries go through `app.platform()`, and one built without a live platform answers nothing.

- Change: the callbacks Flutter assigns onto `PlatformDispatcher` (`onPlatformBrightnessChanged`, `onLocaleChanged`) are `App::platform_callbacks`, which `WidgetsBinding::instance` fills and the shell invokes.
  Reason: platform — there is no isolate-global dispatcher to assign a handler onto.
  Affect: a host reports the change through its client and never calls a binding.

- Change: `App::set_open_requested_handler` takes the app's one handler for what the system asks it to open, and `App::open_requested` keeps each request that comes before a handler is set, handing them over in order as it is set; Flutter's app hears a launch's arguments in `main` and later ones through a plugin.
  Reason: platform — there are no plugins, and the host hands the launch's request on as soon as the client exists, which is before the app has set anything up.
  Affect: an app sets the handler during its setup and still hears the request it was launched with.

- Change: an app reports the files it reads from its own command line, or that a later launch passed on, through the same `App::open_requested`, as a request it builds.
  Reason: platform — a command line holds only strings, and which of them are files is the app's to say, so no host reads one.
  Affect: the files a command line names reach the same handler as the system's own requests, once the app has decided they are files.

- Change: `App::set_notification_activated_handler` takes the app's one handler for the clicks on its notifications, and `App::notification_activated` keeps each click that comes before a handler is set, handing them over in order as it is set, as open requests wait; Flutter's `flutter_local_notifications` hands a running app's clicks to its callback and the launch's to `getNotificationAppLaunchDetails`.
  Reason: platform — there are no plugins, and the host hands the click that started the app on as soon as the client exists, which is before the app has set anything up.
  Affect: an app sets the handler during its setup and still hears the click it was launched by.

- Change: `App::run_in_background` hands a closure to the host's dispatcher and answers its result as a `Task`: dart:isolate's `Isolate.run`.
  Reason: language — there are no isolates; the host's threads and a task are the Rust shape, and the work's panic resumes in the awaiting task as the isolate's error would rethrow.
  Affect: blocking work — a request, a file read — leaves the main thread for the system's queue and comes back through the ordinary checkpoint.

## key.rs → key.dart

- Change: `Key` equality and hashing go through `eq_key` / `hash_key`, so `dyn Key` can be `PartialEq` and `Hash`; a `UniqueKey`'s identity is a monotonic id, not object identity.
  Reason: language — `PartialEq` and `Hash` are not object-safe, and a Rust value type has no implicit object identity.
  Affect: two `UniqueKey::new` calls are never equal; copying a `UniqueKey` keeps the same id.

## constants.rs → constants.dart

- Change: `kProfileMode` is the constant `false`.
  Reason: language — `cfg(debug_assertions)` is the only build distinction Rust exposes, and it splits two ways where Dart splits three.
  Affect: Dart's `kProfileMode` branches never run.

- Change: `kIsWeb` is true for a browser's WebAssembly alone (`wasm32-unknown-unknown`), and `kIsWasm` for every WebAssembly build, a component another host runs included.
  Reason: platform — a WebAssembly component runs outside any browser too, as a WaOS program does, while Rust has no JS-compilation path.
  Affect: a WaOS program's widgets act as the native app of the platform its host reports: text fields take their editing and clipboard keys, and buttons show the arrow; `kIsWeb && !kIsWasm` is never true.

## date_time.rs → dart:core `DateTime`

- Change: only UTC exists: `DateTime::utc(..)`, `now()` read as UTC, and epoch constructors with no `isUtc`.
  Reason: platform — the standard library has no time zone database, and its duration is unsigned.
  Affect: every `DateTime` reads as UTC whatever the host's zone; local time, `parse` and `difference` are absent, so a difference is a subtraction of `microseconds_since_epoch()`.

## change_notifier.rs → change_notifier.dart

- Change: `remove_listener` matches a `Listener` by identity (`Rc::ptr_eq`, or the `(Handle, function)` pair for `handle_method`).
  Reason: language — Rust closures have no identity; Dart's `VoidCallback` compares by identity and tear-offs canonicalize.
  Affect: an equivalent-looking closure built at the removal site removes nothing and the listener keeps firing; keep the `Listener` that was added, or rebuild the `handle_method` tear-off.

- Change: `remove_listener` on a `Handle` whose entry is gone is a no-op.
  Reason: language — Dart allows `removeListener` on a disposed `ChangeNotifier`, whose object the collector still holds; here a remover that names it by a bare handle may find it freed, and a stale `app.get_mut` panics.
  Affect: a state that unmounts after its listenable (a route's `TickerMode`, during a hero flight's pop) removes its listener without panicking; a handle that was never valid is ignored too.

- Change: a panicking listener ends the notification, and `dispose` also clears the reentrant-removal count.
  Reason: language — Rust has no catchable exception for ordinary control flow, so a panic skips the compaction Dart's `catch` always reaches.
  Affect: a panicking listener skips every listener after it; Flutter still calls them.

- Change: a `Listenable` held as a value is an `Rc<dyn Listenable>`, and an arena object is one as a `Retained` (a `Ref`, or an erased reference such as an animation's), never as a bare `Handle`.
  Reason: language — no collector: Dart's reference to a listenable keeps it alive, and a bare handle keeps nothing.
  Affect: whatever holds the `Rc` keeps the object it listens to, so an `AnimatedWidget` whose owner let go of the animation it handed down still removes its listener from it.

- Change: `Listener::downgrade` gives a `WeakListener`, which does not keep the listener's closure, for a closure that removes itself.
  Reason: language — a Rust closure cannot name itself, and one that held its own listener strongly would be a ring of `Rc`s that outlives its removal, where Dart's `late` local names the closure and the collector frees it.
  Affect: a self-removing listener holds a weak copy of itself and removes what that upgrades to; once nothing else holds the listener, the upgrade answers `None`.

## Deferred

- date_time.rs: local time, `parse` / `tryParse`, `difference`. Trigger: a host with a time zone database; a signed duration.
- `ChangeNotifier.maybeDispatchObjectCreation` / `memory_allocations`. Trigger: leak tracker / devtools.
- Diagnostics / `FlutterError` structured trees; messages are `debug_assert!` strings meanwhile. Trigger: porting diagnostics.
- `IterableFilter`. Trigger: the first iterable-filter call site.
- Waking a task from another thread; it would need a `Send` poke into the host's event loop. Trigger: the first background executor or platform callback off the main thread.
