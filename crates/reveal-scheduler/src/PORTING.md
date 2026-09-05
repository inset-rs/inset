# reveal-scheduler/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/scheduler
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- priority.rs → priority.dart

## binding.rs → binding.dart

- Change: `scheduleFrame` asks the host for one frame for the whole app (`app.platform().request_frame()`), not a redraw on every view.
  Reason: platform — the scheduler has one frame; the host chooses the native vsync source.
  Affect: a host implements `Platform::request_frame`.

- Change: `FrameCallback` (and `TickerCallback`) receives `&mut App`.
  Reason: language — a Rust closure cannot capture what it mutates; same as `Listener`.
  Affect: register `FrameCallback::new(|app, time_stamp| …)`; cancel a transient callback by the id `schedule_frame_callback` returned, not by the callback value.

- Change: a panicking frame callback ends that phase; the `finally` still advances `schedulerPhase` (a `Drop` guard).
  Reason: language — Rust has no catchable exception for ordinary control flow, where Dart's `_invokeFrameCallback` catches and continues.
  Affect: a panicking callback skips every callback after it in that phase; Flutter still calls them.

## Handles

- Change: `Ticker` and `TickerFuture` are arena objects whose methods take `self: Handle<Self>`, per the receiver rule in `reveal-foundation/src/PORTING.md` (app.rs).
  Reason: language — see the foundation entry.
  Affect: a crate that calls these inherent methods needs `#![feature(arbitrary_self_types)]`.

## ticker.rs → ticker.dart

- Change: `TickerFuture` runs its registered callbacks through `App::schedule_microtask`; it is not a Rust `Future`.
  Reason: language — Dart's `TickerFuture` implements `Future<void>`, whose listeners the isolate runs on the microtask queue, and a Rust `Future` would need `&mut App` at poll time.
  Affect: write `future.when_complete(app, Listener::new(..))` for `future.whenComplete(cb)` or `.then`; the callback runs at the next drain, never during the `stop()` that resolved the future. Registering `when_complete` on an already-canceled future silently never fires, which is Dart's primary future hanging.

- Change: `TickerProviderObject` is `TickerProvider` for an object in the App (`create_ticker(self: Handle<Self>, ..)`), with the blanket `impl TickerProvider for Handle<T>`; Dart's `vsync: this` on a `State` is the state's handle.
  Reason: language — a downstream crate cannot implement the foreign `TickerProvider` for `Handle<ItsState>` (orphan rule), so the twin lives here.
  Affect: a state writes `impl TickerProviderObject for MyState { .. }` and passes `self` as `vsync`.

## Deferred

- `scheduleTask` and the priority task queue. Trigger: a `Timer` counterpart — `_ensureEventLoopCallback` needs `Timer.run`.
- `_handleBeginFrame` / `_handleDrawFrame` warm-up-frame guards. Trigger: `scheduleWarmUpFrame`.
- `endOfFrame`. Trigger: `RendererBinding.performReassemble` (`rendering/binding.dart`); it hands out a bare `Future`.
- The app lifecycle (`lifecycleState`, `handleAppLifecycleStateChanged`, the `framesEnabled` setter — the field stays `true`). Trigger: the services layer, where lifecycle messages arrive.
- `requestPerformanceMode` and `PerformanceModeRequestHandle`. Trigger: an engine to request modes from.
- Platform dispatcher timings callbacks, timeline, service extensions, debug stack traces (`_FrameCallbackEntry.debugStack`, `debugPrintTransientCallbackRegistrationStack`, `debugLabel` / `toString` / `describeForError`). Trigger: diagnostics.
- `TickerFuture.orCancel` and `TickerCanceled`. Trigger: the first ported `await …orCancel` — nothing under `packages/flutter/lib` uses it; material uses `whenCompleteOrCancel`.
