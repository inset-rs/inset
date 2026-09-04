# reveal-gestures/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/gestures
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- constants.rs → constants.dart
- gesture_settings.rs → gesture_settings.dart
- gesture_details.rs → gesture_details.dart
- recognizer.rs → OffsetPair, DragStartBehavior, MultitouchDragStrategy, GestureRecognizerState
- tap.rs → TapDownDetails / TapUpDetails / TapMoveDetails
- velocity.rs → Velocity / VelocityEstimate
- lsq_solver.rs → lsq_solver.dart
- long_press.rs → LongPressDownDetails / LongPressStartDetails / LongPressMoveUpdateDetails / LongPressEndDetails

## Handle receivers

- Change: [`GestureArenaManager`](GestureArenaManager), [`GestureArenaTeam`](GestureArenaTeam), [`PointerRouter`](PointerRouter), [`GestureBinding`](GestureBinding), [`TapGestureRecognizer`](TapGestureRecognizer) and [`LongPressGestureRecognizer`](LongPressGestureRecognizer) are arena objects: `new(app, ..)` returns `Handle<T>` and methods take `self: Handle<Self>` plus [`App`](reveal_foundation::App), per the `Handle<T>` receiver rule in `reveal-foundation/src/PORTING.md` (app.rs).
  Reason: language — see that entry.
  Affect: `let tap = TapGestureRecognizer::new(app)`, then `tap.add_pointer(app, down)`; `GestureBinding::instance(app).handle_pointer_event(app, event)`. Callbacks are [`Listener`](reveal_foundation::Listener) / [`ValueChanged`](reveal_foundation::ValueChanged) and receive `&mut App`.

## events.rs → events.dart

- Change: [`PointerEvent`](PointerEvent) is a pairing enum over the public event classes. `_Transformed*` subclasses are the same struct with [`transform`](PointerDownEvent::transform) set.
  Reason: language — Rust has no inheritance for a stored abstract type whose kind is the public subclasses.
  Affect: store `PointerEvent::Down(...)`. `is PointerDownEvent` is `matches!(event, PointerEvent::Down(_))`.

- Change: `viewId` is [`ViewId`](reveal_embedder::ViewId), not a bare `int`.
  Reason: platform — views are already `ViewId` on this host surface, same as [`PointerData`](reveal_embedder::PointerData).
  Affect: fill `view_id: view.id()`, not a raw integer.

- Change: `respond` / `onRespond` are omitted on signal events.
  Reason: platform — they exist to call `preventDefault` on the web DOM event that produced the sample, same as `PointerData.respond`.
  Affect: there is no `event.respond(...)`.

- Change: `PointerSignalEventListener` receives the `PointerEvent` enum.
  Reason: language — `PointerScrollEvent`, `PointerScrollInertiaCancelEvent`, and `PointerScaleEvent` are enum variants with no shared `PointerSignalEvent` type.
  Affect: match `PointerEvent::Scroll` / `ScrollInertiaCancel` / `Scale` in the listener.

## binding.rs → binding.dart

- Change: [`GestureBinding::instance`](GestureBinding::instance) is the App singleton.
  Reason: language — same as [`SchedulerBinding::instance`](reveal_scheduler::SchedulerBinding::instance); Rust has no mixin-on-one-object.
  Affect: `GestureBinding::instance(app)` where Dart writes `GestureBinding.instance`.

- Change: [`pointer_router`](GestureBinding::pointer_router) / [`gesture_arena`](GestureBinding::gesture_arena) return `Handle`s created on first [`instance`](GestureBinding::instance).
  Reason: language — `App::singleton` `Default` cannot mint child Handles; the fields are filled on first access.
  Affect: `let router = binding.pointer_router(app); router.add_route(app, ...)`.

- Change: `GestureBinding::hit_test_in_view` and `dispatch_event` call a registered `GestureBindingOverrides` first (walk the render trees; feed the mouse tracker), then run their own body. `RendererBinding` registers itself with [`set_overrides`](GestureBinding::set_overrides).
  Reason: language — Flutter's `RendererBinding` overrides those methods through mixin order; a crate above cannot override a method below it.
  Affect: none for callers; a test binding that hit-tests its own tree registers a `GestureBindingOverrides` with `GestureBinding::instance(app).set_overrides(app, overrides)`.

- Change: [`GestureBindingOverridesObject`](GestureBindingOverridesObject) is the object side of [`GestureBindingOverrides`](GestureBindingOverrides): implement it on the binding type, and `Handle<T>` is then [`HitTestable`](HitTestable) and `GestureBindingOverrides`.
  Reason: language — orphan rule; a crate above cannot implement those foreign `&self` traits for `Handle<ItsType>`.
  Affect: `impl GestureBindingOverridesObject for RendererBinding`, then register `Rc::new(handle)`.

- Change: `dispatch_event` / `_handlePointerDataPacket` do not catch panics or report `FlutterError`.
  Reason: language — no catchable exception; diagnostics are deferred.
  Affect: a panicking target skips the rest of the path.

## converter.rs → converter.dart

- Change: `devicePixelRatioForView` takes [`ViewId`](reveal_embedder::ViewId).
  Reason: platform — same as [`PointerData::view_id`](reveal_embedder::PointerData::view_id).
  Affect: the getter is `Fn(ViewId) -> Option<f64>`.

## debug.rs → debug.dart

- Change: `debug_print_*` / `set_debug_print_*` instead of assigning a library `bool`.
  Reason: language — Rust has no isolate-global assignable `bool` binding.
  Affect: call the setter; the getter is the Dart read.

## hit_test.rs → hit_test.dart

- Change: [`HitTestable`](HitTestable) / [`HitTestDispatcher`](HitTestDispatcher) / [`HitTestTarget`](HitTestTarget) methods take [`App`](reveal_foundation::App).
  Reason: language — a Rust callback cannot capture what it mutates.
  Affect: `target.handle_event(app, event, entry)`.

- Change: `viewId` is [`ViewId`](reveal_embedder::ViewId).
  Reason: platform — same as [`PointerData::view_id`](reveal_embedder::PointerData::view_id).
  Affect: `hit_test_in_view(..., view.id())`.

- Change: there is no `HitTestResult.wrap`. Hit-test methods take `&mut HitTestResult`.
  Reason: language — Dart's wrap is two objects over one list; a Rust `Vec` has one owner.
  Affect: pass `&mut result` where Dart writes `HitTestResult.wrap(result)`.

- Change: [`add`](HitTestResult::add) takes the entry by value and writes `transform` on the stored copy.
  Reason: language — Dart mutates the same object the caller still holds.
  Affect: after `result.add(entry)`, `entry` is gone. Read `result.path().last().transform()`.

## arena.rs → arena.dart

- Change: members compared by [`member_id`](GestureArenaMember::member_id) (`HandleId`).
  Reason: language — Rust has no object identity for a trait object.
  Affect: implement [`GestureArenaMember`](GestureArenaMember) for `Handle<T>` and return `self.id()`.

## team.rs → team.dart

- Change: [`captain`](GestureArenaTeam::captain) returns the captain's `HandleId`; [`set_captain`](GestureArenaTeam::set_captain) takes the member.
  Reason: language — `captain` is a `GestureArenaMember` field, not a typed Handle, and a trait object cannot be handed back by value.
  Affect: `team.set_captain(app, member)`; `team.captain(app)` is an `Option<HandleId>`.

## pointer_router.rs → pointer_router.dart

- Change: [`PointerRoute`](PointerRoute) receives [`App`](reveal_foundation::App). Identity is [`Listener`](reveal_foundation::Listener)-shaped (`new` clones, or [`handle_method`](PointerRoute::handle_method)).
  Reason: language — Rust closures have no identity; Dart's `PointerRoute` compares by identity and tear-offs canonicalize.
  Affect: `PointerRoute::new(|app, event| …)` or `PointerRoute::handle_method(this, Self::handle_event)`. Keep the value, or rebuild the tear-off at `remove_route`.

- Change: `_dispatch` does not catch panics or report `FlutterError`.
  Reason: language — no catchable exception for ordinary control flow; diagnostics are deferred.
  Affect: a panicking route skips every route after it. Flutter reports and continues.

## recognizer.rs / tap.rs / long_press.rs — GestureRecognizer hierarchy

Pattern: [leaf-inheritance](../../../.cursor/skills/porting-flutter/patterns/leaf-inheritance.md).

- Change: `invokeCallback` does not catch panics or report `FlutterError`.
  Reason: language — no catchable exception; diagnostics are deferred.
  Affect: a panicking `onTap` / `onLongPress` unwinds instead of logging and continuing.

- Change: Dart's `GestureRecognizer` used as a type (a field, a `Map<Type, GestureRecognizer>` value) is the erased [`AnyGestureRecognizer`](AnyGestureRecognizer): `id` plus a `&'static` vtable, minted by [`as_recognizer`](GestureRecognizerLeaf::as_recognizer) on every leaf. [`type_id`](AnyGestureRecognizer::type_id) is Dart's `runtimeType`; [`downcast`](AnyGestureRecognizer::downcast) is `recognizer as T`.
  Reason: language — no inheritance; an arena id has no fat pointer for a `dyn` trait.
  Affect: store `tap.as_recognizer()` where Dart stores a `GestureRecognizer`; it has `add_pointer` / `add_pointer_pan_zoom` (taking `&event`), `is_pointer_allowed`, `dispose`, `debug_description`. Bound a generic by [`GestureRecognizerLeaf`](GestureRecognizerLeaf) where Dart writes `T extends GestureRecognizer`.

- Change: `dispose` frees the arena slot after Dart's body, on the typed handle and the erased edge alike.
  Reason: language — Dart leaves the disposed object to the collector; the arena has none.
  Affect: every handle and edge to the recognizer is stale after `dispose`; calling it twice panics.

## velocity_tracker.rs → velocity_tracker.dart

- Change: [`VelocityTracker`](VelocityTracker) `_sinceLastSample` is `Instant::now()`, not `GestureBinding.samplingClock.stopwatch()`.
  Reason: language — `SamplingClock` is deferred with resampling; there is no isolate `Stopwatch` tied to FakeAsync.
  Affect: the 40ms “pointer stopped” path follows wall time, not [`App::elapse`](reveal_foundation::App::elapse).

## Deferred

- `PointerEnterEvent` / `PointerExitEvent`. Trigger: `MouseTracker`.
- `NativeHitTestTarget`. Trigger: platform views.
- `debugOwner` / `debugFillProperties` on recognizers. Trigger: diagnostics (F3).
- `_Resampler` / `SamplingClock` / `resamplingEnabled`. Trigger: a host that wants touch resampling.
- `PointerSignalResolver`. Trigger: scroll/wheel.
- Engine `onHitTest`. Trigger: platform views.
