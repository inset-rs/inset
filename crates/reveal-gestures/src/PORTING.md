# reveal-gestures/src
Flutter home: packages/flutter/lib/src/gestures
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- constants.rs → constants.dart
- gesture_settings.rs → gesture_settings.dart
- gesture_details.rs → gesture_details.dart
- drag_details.rs → drag_details.dart
- events.rs → computeHitSlop / computePanSlop
- recognizer.rs → OffsetPair, DragStartBehavior, MultitouchDragStrategy, GestureRecognizerState
- tap.rs → TapDownDetails / TapUpDetails / TapMoveDetails
- velocity.rs → Velocity / VelocityEstimate
- lsq_solver.rs → lsq_solver.dart
- long_press.rs → LongPressDownDetails / LongPressStartDetails / LongPressMoveUpdateDetails / LongPressEndDetails
- debug.rs → debug.dart
- tap_and_drag.rs → TapDragDownDetails / TapDragUpDetails / TapDragStartDetails / TapDragUpdateDetails / TapDragEndDetails
- force_press.rs → ForcePressDetails

## events.rs → events.dart

- Change: `PointerEvent` is one enum over the public event classes, and a transformed event is the same value with `transform` set rather than a wrapper around an original.
  Reason: language — Rust has no inheritance for a stored abstract type whose kind is the public subclasses.
  Affect: there is no shared `PointerSignalEvent` type, so a signal listener matches `Scroll` / `ScrollInertiaCancel` / `Scale`.

- Change: `respond` / `onRespond` are absent on signal events.
  Reason: platform — they exist to call `preventDefault` on the web DOM event that produced the sample.
  Affect: nothing can tell the host that a signal was or was not handled.

## binding.rs → binding.dart

- Change: `hit_test_in_view` and `dispatch_event` call a registered `GestureBindingOverrides` before their own body, where Flutter's `RendererBinding` overrides them through mixin order.
  Reason: language — a crate above cannot override a method below it, and the orphan rule blocks implementing the foreign traits for its own handle.
  Affect: a binding that forgets `set_overrides` hit-tests nothing and never feeds the mouse tracker; ordinary callers see the Flutter order.

- Change: neither the binding's dispatch, the router's `_dispatch` nor a recognizer's `invokeCallback` catches panics.
  Reason: language — no catchable exception and no `FlutterError.reportError` hook.
  Affect: a panicking target, route or `onTap` unwinds and the rest of that dispatch is skipped, where Flutter reports and continues.

## hit_test.rs → hit_test.dart

- Change: `HitTestResult::add` takes the entry by value and writes `transform` on the stored copy.
  Reason: language — Dart writes it on the same object the caller still holds.
  Affect: a caller cannot read the transform back off the entry it added; read `result.path().last().transform()`.

## pointer_signal_resolver.rs → pointer_signal_resolver.dart

- Change: `register` keeps the event it was handed and `resolve` reads that one back; Dart's `_isSameEvent` assertions are gone.
  Reason: language — `PointerEvent` has no `==` for those assertions to compare with.
  Affect: registering for a second event before resolving the first silently keeps the first, where Dart asserts.

- Change: `resolve` does nothing when no callback registered.
  Reason: platform — `PointerEvent.respond` is absent on signal events (see `events.rs`).
  Affect: an unhandled scroll never tells the host to run its default action.

## monodrag.rs → monodrag.dart

Pattern: [leaf-inheritance](../../../.agents/skills/porting-flutter/patterns/leaf-inheritance.md). `DragGestureRecognizer` is a trait blanket-implemented for the three axis leaves, which carry the `GestureRecognizer` and `OneSequence` bags only; a drag subclass outside this crate holds those bags and calls the shared bodies where Dart writes `super`.

## tap_and_drag.rs → tap_and_drag.dart

Pattern: same leaf-inheritance as [monodrag.rs](#monodragrs--monodragdart). `BaseTapAndDragGestureRecognizer` is a trait blanket-implemented for the two axis leaves; `_TapStatusTrackerMixin` is `TapStatusTrackerData` plus `TapStatusTracker`, held on `TapAndDragData`.

## multidrag.rs → multidrag.dart

Pattern: same leaf-inheritance. Each per-pointer state is an arena object, and a custom recognizer implements `MultiDragGestureRecognizer` and a `MultiDragPointerState` leaf.

## recognizer.rs / tap.rs / long_press.rs / monodrag.rs / tap_and_drag.rs / force_press.rs — GestureRecognizer hierarchy

Pattern: [leaf-inheritance](../../../.agents/skills/porting-flutter/patterns/leaf-inheritance.md). The field bags, leaf traits and `super` namespaces of every level are public, since a leaf written in another crate has to name each piece it would have inherited; `AnyGestureRecognizer` is the erased form Dart uses `GestureRecognizer` as a type for.

- Change: `dispose` frees the arena slot after Dart's body, on the typed and the erased handle alike.
  Reason: language — Dart leaves the disposed object to the collector; the arena has none.
  Affect: every handle to the recognizer is stale after `dispose`, and disposing twice panics instead of being ignored.

## velocity_tracker.rs → velocity_tracker.dart

- Change: `VelocityTracker` measures the time since the last sample with `Instant::now()`, not `GestureBinding.samplingClock`.
  Reason: platform — the sampling clock ships with resampling, which is deferred, and nothing ties it to the app clock.
  Affect: the 40ms "pointer stopped" rule follows wall time, so `App::elapse` does not move it in a test.

## Deferred

- `PointerEnterEvent` / `PointerExitEvent`. Trigger: `MouseTracker`.
- `NativeHitTestTarget`, engine `onHitTest`. Trigger: platform views.
- `debugOwner` / `debugFillProperties` on recognizers. Trigger: diagnostics (F3).
- `_Resampler` / `SamplingClock` / `resamplingEnabled`. Trigger: a host that wants touch resampling.
- `TapAndDragGestureRecognizer` (deprecated alias of `TapAndPanGestureRecognizer`). Trigger: a caller written against the old name.
