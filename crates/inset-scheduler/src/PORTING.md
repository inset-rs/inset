# inset-scheduler/src
Flutter home: packages/flutter/lib/src/scheduler
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- priority.rs → priority.dart
- binding.rs `handle_app_lifecycle_state_changed` → `SchedulerBinding.handleAppLifecycleStateChanged`, which the widgets binding's override extends to tell its observers

## binding.rs → binding.dart

- Change: a frame is a refresh of the views on one screen that asked for one, and every request for a frame names one view or every view, where Flutter's one frame is of every view.
  Reason: platform — the host refreshes each screen at its own rate and gives a minimized or occluded window no frames.
  Affect: a change shows in a window at that window's next frame, at its screen's rate, and not while it is minimized or occluded.

- Change: a transient or post-frame callback runs at the first frame of its view, or of any view when it has none, where Flutter's one frame runs them all.
  Reason: platform — a view gets frames of its own, and work above every view, such as the image cache's or the mouse tracker's, belongs to none.
  Affect: a widget's animation and after-frame work wait for its own window's frames, while the app's own runs at any window's.

- Change: a frame's time stamp is never earlier than the last frame's.
  Reason: platform — two screens refresh at times out of order.
  Affect: an animation never steps back when frames of windows on different screens interleave.

- Change: work for a view the platform no longer has counts as work of no view.
  Reason: platform — a host gives no frames to a view it no longer has.
  Affect: work left for a window that closed runs at another window's next frame rather than never.

- Change: a panicking frame callback ends that phase; a `Drop` guard still advances `scheduler_phase`.
  Reason: language — Rust has no catchable exception where Dart's `_invokeFrameCallback` catches and continues.
  Affect: a panicking callback skips every callback after it in that phase; Flutter still calls them.

## ticker.rs → ticker.dart

- Change: a ticker's view follows its widget when a `GlobalKey` moves it to another view, and a tick then due moves with it.
  Reason: platform — a ticker ticks at its own view's frames, and a `GlobalKey` can move its widget to another view.
  Affect: an animation keeps running when its widget moves to another window, though the window it left is hidden.

- Change: `TickerFuture` is a `Future<Output = ()>`; the cancellation Dart throws is the `Err` of `or_cancel`, and `when_complete` / `when_complete_or_cancel` queue their callback on the App's microtask queue.
  Reason: language — a Rust future has no error channel, and a queued callback needs the App.
  Affect: the callback runs at the next microtask drain, never inside the `stop()` that resolved the future, and never at all on a canceled ticker; a task awaiting `or_cancel()` matches on a `Result` where Dart catches `TickerCanceled`.

## Deferred

- `scheduleTask` and the priority task queue. Trigger: a `Timer` counterpart for `_ensureEventLoopCallback`.
- `lockEvents` around the warm-up frame. Trigger: a host that takes events between two of the framework's own tasks.
- `endOfFrame`. Trigger: `RendererBinding.performReassemble`.
- `resetInternalState`. Trigger: a test binding that resets the lifecycle between tests.
- `requestPerformanceMode` and `PerformanceModeRequestHandle`. Trigger: an engine to request modes from.
- Frame-timing callbacks, timeline, service extensions and debug stack traces. Trigger: diagnostics.
