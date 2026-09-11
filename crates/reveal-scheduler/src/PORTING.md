# reveal-scheduler/src
Flutter home: packages/flutter/lib/src/scheduler
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- priority.rs → priority.dart

## binding.rs → binding.dart

- Change: `schedule_frame` asks the host for one frame for the whole app, where Dart schedules a redraw per view.
  Reason: platform — the host owns a single vsync source.
  Affect: a host with several views draws them all on one frame, and cannot be asked to refresh just one.

- Change: a panicking frame callback ends that phase; a `Drop` guard still advances `scheduler_phase`.
  Reason: language — Rust has no catchable exception where Dart's `_invokeFrameCallback` catches and continues.
  Affect: a panicking callback skips every callback after it in that phase; Flutter still calls them.

## ticker.rs → ticker.dart

- Change: `TickerFuture` is a `Future<Output = ()>`; the cancellation Dart throws is the `Err` of `or_cancel`, and `when_complete` / `when_complete_or_cancel` queue their callback on the App's microtask queue.
  Reason: language — a Rust future has no error channel, and a queued callback needs the App.
  Affect: the callback runs at the next microtask drain, never inside the `stop()` that resolved the future, and never at all on a canceled ticker; a task awaiting `or_cancel()` matches on a `Result` where Dart catches `TickerCanceled`.

## Deferred

- `scheduleTask` and the priority task queue. Trigger: a `Timer` counterpart for `_ensureEventLoopCallback`.
- Warm-up-frame guards in `_handleBeginFrame` / `_handleDrawFrame`. Trigger: `scheduleWarmUpFrame`.
- `endOfFrame`. Trigger: `RendererBinding.performReassemble`.
- The app lifecycle (`lifecycleState`, `handleAppLifecycleStateChanged`, the `framesEnabled` setter). Trigger: the services layer, where lifecycle messages arrive.
- `requestPerformanceMode` and `PerformanceModeRequestHandle`. Trigger: an engine to request modes from.
- Frame-timing callbacks, timeline, service extensions and debug stack traces. Trigger: diagnostics.
