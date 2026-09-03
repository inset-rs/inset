# reveal-gestures/src
Flutter home: packages/flutter/lib/src/gestures
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- constants.rs → constants.dart
- gesture_settings.rs → gesture_settings.dart
- gesture_details.rs → gesture_details.dart
- recognizer.rs → OffsetPair, DragStartBehavior, MultitouchDragStrategy, GestureRecognizerState
- tap.rs → TapDownDetails / TapUpDetails / TapMoveDetails

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

- Change: [`GestureArenaManager`](GestureArenaManager) is a Handle newtype. [`add`](GestureArenaManager::add) / [`close`](GestureArenaManager::close) / [`sweep`](GestureArenaManager::sweep) / [`hold`](GestureArenaManager::hold) / [`release`](GestureArenaManager::release) and [`GestureArenaEntry::resolve`](GestureArenaEntry::resolve) take [`App`](reveal_foundation::App).
  Reason: language — callbacks must re-enter App; sole-member win uses [`App::schedule_microtask`](reveal_foundation::App::schedule_microtask). A `&mut` of the tables cannot be held across `accept_gesture`.
  Affect: `GestureArenaManager::new(app)`. `arena.close(app, pointer)`. `entry.resolve(app, disposition)`.

- Change: members compared by [`member_id`](GestureArenaMember::member_id) (`HandleId`).
  Reason: language — Rust has no object identity for a trait object.
  Affect: implementors are Handle newtypes; return `self.0.id()`.

## team.rs → team.dart

- Change: [`GestureArenaTeam`](GestureArenaTeam) is a Handle newtype. [`add`](GestureArenaTeam::add) takes [`App`](reveal_foundation::App). [`captain`](GestureArenaTeam::captain) is `captain` / [`set_captain`](GestureArenaTeam::set_captain).
  Reason: language — the team mutates App-owned combiner slots; `captain` is a `GestureArenaMember` field, not a typed Handle.
  Affect: `GestureArenaTeam::new(app)`. `team.add(app, pointer, member)`. `team.set_captain(app, member)`.

## pointer_router.rs → pointer_router.dart

- Change: [`PointerRouter`](PointerRouter) is a Handle newtype. Route methods take [`App`](reveal_foundation::App).
  Reason: language — `route` snapshots the tables, drops the slot, then calls; a route may `remove_route` the same router.
  Affect: `PointerRouter::new(app)`. `router.add_route(app, pointer, route, transform)`. `router.route(app, event)`.

- Change: [`PointerRoute`](PointerRoute) receives [`App`](reveal_foundation::App). Identity is [`Listener`](reveal_foundation::Listener)-shaped (`new` clones, or [`handle_method`](PointerRoute::handle_method)).
  Reason: language — Rust closures have no identity; Dart's `PointerRoute` compares by identity and tear-offs canonicalize.
  Affect: `PointerRoute::new(|app, event| …)` or `PointerRoute::handle_method(this, Self::handle_event)`. Keep the value, or rebuild the tear-off at `remove_route`.

- Change: `_dispatch` does not catch panics or report `FlutterError`.
  Reason: language — no catchable exception for ordinary control flow; diagnostics are deferred.
  Affect: a panicking route skips every route after it. Flutter reports and continues.

## binding.rs → binding.dart

- Change: [`GestureBinding::instance`](GestureBinding::instance) is the App singleton. Members take `&mut App`.
  Reason: language — same as [`SchedulerBinding::instance`](reveal_scheduler::SchedulerBinding::instance); Rust has no mixin-on-one-object.
  Affect: `GestureBinding::instance(app)` where Dart writes `GestureBinding.instance`.

- Change: [`pointer_router`](GestureBinding::pointer_router) / [`gesture_arena`](GestureBinding::gesture_arena) return Copy handles created on first [`instance`](GestureBinding::instance).
  Reason: language — `App::singleton` `Default` cannot mint child Handles; the fields are filled on first access.
  Affect: `let router = binding.pointer_router(app); router.add_route(app, ...)`.

- Change: `hit_test_in_view` always adds this binding only. There is no override from `RendererBinding`.
  Reason: language — Rust has no mixin override across crates.
  Affect: a down event's path is `[GestureBinding]` until `RendererBinding` exists.

- Change: `dispatch_event` / `_handlePointerDataPacket` do not catch panics or report `FlutterError`.
  Reason: language — no catchable exception; diagnostics are deferred.
  Affect: a panicking target skips the rest of the path.

## recognizer.rs / tap.rs — GestureRecognizer hierarchy

One Handle on the leaf ([`TapGestureRecognizer`](TapGestureRecognizer)). Superclasses are field bags. `super` is an associated fn on a namespace, called inside the body at Dart's position. Virtuals from a superclass body call the leaf method. A second leaf must not hard-code `BaseTap` inside `PrimaryPointer`. Pattern file on the second leaf.

- Change: [`TapGestureRecognizer`](TapGestureRecognizer) is a Handle newtype. Methods take [`App`](reveal_foundation::App). Callbacks receive [`App`].
  Reason: language — same as other Handle newtypes; a Rust callback cannot capture what it mutates.
  Affect: `TapGestureRecognizer::new(app)`. `tap.add_pointer(app, down)`. `tap.set_on_tap(app, |app| …)`.

- Change: `invokeCallback` does not catch panics or report `FlutterError`.
  Reason: language — no catchable exception; diagnostics are deferred.
  Affect: a panicking `onTap` unwinds instead of logging and continuing.

## Deferred

- `PointerEnterEvent` / `PointerExitEvent`. Trigger: `MouseTracker`.
- `NativeHitTestTarget`. Trigger: platform views.
- `debugOwner` / `debugFillProperties` on recognizers. Trigger: diagnostics (F3).
- `_Resampler` / `SamplingClock` / `resamplingEnabled`. Trigger: a host that wants touch resampling.
- `PointerSignalResolver`. Trigger: scroll/wheel.
- Engine `onHitTest`. Trigger: platform views.
