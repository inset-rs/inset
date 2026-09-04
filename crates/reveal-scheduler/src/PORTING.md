# reveal-scheduler/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/scheduler
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- priority.rs → priority.dart

## binding.rs → binding.dart

- Change: `SchedulerBinding.instance` is `SchedulerBinding::instance(app)`, the App's singleton, and every member takes `&mut App`: `SchedulerBinding::schedule_frame(app)` for `SchedulerBinding.instance.scheduleFrame()`.
  Reason: language — Dart's binding is a per-isolate global; the `&mut App` in hand is the ambient authority, and `App::singleton` is the per-App counterpart of the per-isolate instance.
  Affect: when porting a `SchedulerBinding.instance.x()` call, write `SchedulerBinding::x(app)`.

- Change: `scheduleFrame` asks the host for one frame for the whole app (`app.platform().request_frame()`), not a redraw on every view.
  Reason: platform — the scheduler has one frame; the host chooses the native vsync source.
  Affect: a host implements `Platform::request_frame`.

- Change: `FrameCallback` (and `TickerCallback`) receives `&mut App`.
  Reason: language — a Rust closure cannot capture what it mutates; same as `Listener`.
  Affect: register `FrameCallback::new(|app, time_stamp| …)`. Cancel a transient callback by the id `schedule_frame_callback` returned, not by the callback value.

- Change: `timeDilation` is `SchedulerBinding::time_dilation(app)` / `set_time_dilation(app, v)`, not a global.
  Reason: language — Dart keeps it as a mutable top-level; Rust has no mutable global without `unsafe` or a lock, and the binding is where every reader already looks.
  Affect: write `SchedulerBinding::set_time_dilation(app, 2.0)` where Flutter assigns `timeDilation = 2.0`. The setter resets the epoch exactly as Dart's does.

- Change: a panicking frame callback ends that phase; remaining callbacks in the phase are not called. The `finally` still advances `schedulerPhase` (Drop guard).
  Reason: language — Rust has no catchable exception for ordinary control flow. Dart's `_invokeFrameCallback` catches and continues.
  Affect: a panicking callback skips every callback after it in that phase. Flutter still calls them.

## Handles

- Change: [`Ticker`] and [`TickerFuture`] are arena objects whose methods take `self: Handle<Self>` — the receiver rule in `reveal-foundation/src/PORTING.md` (app.rs).
  Reason: language — see the foundation entry.
  Affect: `let ticker = Ticker::new(app, on_tick)` is a `Handle<Ticker>`; `ticker.start(app)` returns a `Handle<TickerFuture>`; `future.when_complete(app, listener)`. A crate that calls these inherent methods needs `#![feature(arbitrary_self_types)]`.

## ticker.rs → ticker.dart

- Change: `TickerFuture` runs registered callbacks (`when_complete`, `when_complete_or_cancel`) through `App::schedule_microtask`; it is not a Rust `Future`.
  Reason: language — Dart's `TickerFuture` implements `Future<void>`, whose listeners the isolate runs on the microtask queue. A Rust `Future` would need `&mut App` at poll time.
  Affect: when porting `future.whenComplete(cb)` or `.then`, write `future.when_complete(app, Listener::new(...))`; the callback runs at the next drain, never during the `stop()` that resolved the future. Registering `when_complete` on an already-canceled future silently never fires, which is Dart's primary future hanging.

- Change: `Ticker.muted` is the accessor `muted(app)` and the setter `set_muted(app, value)`.
  Reason: language — the setter schedules and unschedules ticks, so it needs the App.
  Affect: write `ticker.set_muted(app, true)` where Flutter assigns `ticker.muted = true`.

- Change: [`TickerProvider::create_ticker`] takes `self` and `&mut App`.
  Reason: language — a `&mut self` receiver cannot also produce the `&mut App` that creating a ticker needs, and a provider living in the App cannot be borrowed mutably while that App is borrowed.
  Affect: `vsync.create_ticker(app, on_tick)` where Dart writes `vsync.createTicker(onTick)`. Pass a `Handle` (or a Copy test double) by value, not `&mut dyn TickerProvider`.

- Change: `TickerProviderObject` is `TickerProvider` for an object in the App (`create_ticker(self: Handle<Self>, ..)`), with the blanket `impl<T: TickerProviderObject> TickerProvider for Handle<T>`; Dart's `vsync: this` on a `State` is the state's handle.
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
