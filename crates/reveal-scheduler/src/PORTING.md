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

- Change: `Ticker` is an arena object whose methods take `self: Handle<Self>`, per the receiver rule in `reveal-foundation/src/PORTING.md` (app.rs).
  Reason: language — see the foundation entry.
  Affect: a crate that calls these inherent methods needs `#![feature(arbitrary_self_types)]`.

## ticker.rs → ticker.dart

- Change: `TickerFuture` is a value implementing `Future<Output = ()>`, cloned into the ticker and out to the caller; `or_cancel` is a `CompleterFuture<Result<(), TickerCanceled>>` rather than a future that throws, and `when_complete` / `when_complete_or_cancel` take the `App` to queue their callback.
  Reason: language — a Rust future has no error channel, so the cancellation Dart throws is the `Err` of the secondary future; a callback runs on the App's microtask queue, so registering one needs the App (foundation's `then`).
  Affect: `controller.forward(app).await` in a task, or `future.when_complete(app, Listener::new(..))` for `.then` / `.whenComplete`; the callback runs at the next microtask drain, never during the `stop()` that resolved the future, and never at all on a canceled ticker, which is Dart's primary future hanging. A task awaiting `or_cancel()` matches on the `Result` where Dart catches `TickerCanceled`.

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
