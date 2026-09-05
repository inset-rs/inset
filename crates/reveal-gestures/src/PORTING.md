# reveal-gestures/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
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

## Handle receivers

- Change: the arena manager and team, the pointer router, the binding, and the tap and long-press recognizers are arena objects: `new(app, ..)` returns a `Handle` and methods take `self: Handle<Self>` plus the App, per the receiver rule in `reveal-foundation/src/PORTING.md` (app.rs).
  Reason: language — see that entry.
  Affect: callbacks are `Listener` / `ValueChanged` and receive `&mut App`.

## events.rs → events.dart

- Change: `PointerEvent` is a pairing enum over the public event classes; the `_Transformed*` subclasses are the same struct with `transform` set, and a signal listener receives the enum because there is no shared `PointerSignalEvent` type.
  Reason: language — Rust has no inheritance for a stored abstract type whose kind is the public subclasses.
  Affect: a `PointerSignalEventListener` matches `Scroll` / `ScrollInertiaCancel` / `Scale`.

- Change: `respond` / `onRespond` are omitted on signal events.
  Reason: platform — they exist to call `preventDefault` on the web DOM event that produced the sample, as on `PointerData`.
  Affect: there is no `event.respond(..)`.

## binding.rs → binding.dart

- Change: `hit_test_in_view` and `dispatch_event` call a registered `GestureBindingOverrides` first (walk the render trees, feed the mouse tracker), then run their own body; `RendererBinding` registers itself with `set_overrides`. The object-side twin `GestureBindingOverridesObject` is what a binding type implements, and its `Handle` is then `HitTestable` and the overrides.
  Reason: language — Flutter's `RendererBinding` overrides those methods through mixin order; a crate above cannot override a method below it, and the orphan rule keeps it from implementing the foreign `&self` traits for `Handle<ItsType>`.
  Affect: a binding that hit-tests its own tree writes `impl GestureBindingOverridesObject for ItsType` and registers `Rc::new(handle)` with `GestureBinding::instance(app).set_overrides(app, ..)`; ordinary callers see nothing.

- Change: neither the binding's dispatch, the router's `_dispatch` nor a recognizer's `invokeCallback` catches panics or reports a `FlutterError`.
  Reason: language — no catchable exception; diagnostics are deferred.
  Affect: a panicking target, route or `onTap` unwinds and the rest of that path is skipped; Flutter reports and continues.

## hit_test.rs → hit_test.dart

- Change: `HitTestable` / `HitTestDispatcher` / `HitTestTarget` methods take the `App`.
  Reason: language — a Rust callback cannot capture what it mutates.
  Affect: `target.handle_event(app, event, entry)`.

- Change: `HitTestResult::add` takes the entry by value and writes `transform` on the stored copy.
  Reason: language — Dart mutates the same object the caller still holds.
  Affect: after `result.add(entry)`, `entry` is gone; read `result.path().last().transform()`.

## arena.rs → arena.dart

- Change: members are compared by `member_id` (a `HandleId`).
  Reason: language — Rust has no object identity for a trait object.
  Affect: implement `GestureArenaMember` for `Handle<T>` and return `self.id()`.

## team.rs → team.dart

- Change: `captain` returns the captain's `HandleId`; `set_captain` takes the member.
  Reason: language — `captain` is a `GestureArenaMember` field, not a typed Handle, and a trait object cannot be handed back by value.
  Affect: `team.set_captain(app, member)`; `team.captain(app)` is an `Option<HandleId>`.

## pointer_router.rs → pointer_router.dart

- Change: a `PointerRoute` receives the `App`, and its identity is `Listener`-shaped: `new` clones, or `handle_method` for a tear-off.
  Reason: language — Rust closures have no identity; Dart's `PointerRoute` compares by identity and tear-offs canonicalize.
  Affect: `PointerRoute::new(|app, event| …)` or `PointerRoute::handle_method(this, Self::handle_event)`; keep the value, or rebuild the tear-off at `remove_route`.

## drag.rs → drag.dart

- Change: `Drag`'s methods take the `App`; an arena object implements `DragObject`, whose methods take `self: Handle<Self>`, and is held as an `Rc<dyn Drag>` over its handle.
  Reason: language — every Dart object that implements `Drag` has identity and lives in the arena, and a Rust callback cannot capture what it mutates.
  Affect: `impl DragObject for MyActivity`, then `Rc::new(activity)` where Dart passes the object as a `Drag`, and `drag.update(app, details)`.

## pointer_signal_resolver.rs → pointer_signal_resolver.dart

- Change: `register` and `resolve` take the `PointerEvent` enum, `resolve` reads the event it kept rather than taking it again, and Dart's `_isSameEvent` assertions are gone.
  Reason: language — the three signal events are enum variants with no shared type, and `PointerEvent` has no `==` for `_isSameEvent` to compare with.
  Affect: `resolver.register(app, &event, Rc::new(|app, event| ..))` then `resolver.resolve(app)`; registering for a second event before resolving the first silently keeps the first.

- Change: `resolve` does nothing when no callback registered.
  Reason: platform — `PointerEvent.respond` is omitted on signal events (see `events.rs`).
  Affect: an unhandled scroll never tells the host to run its default action.

## monodrag.rs → monodrag.dart

Pattern: [leaf-inheritance](../../../.cursor/skills/porting-flutter/patterns/leaf-inheritance.md). `DragGestureRecognizer` extends `OneSequenceGestureRecognizer`, so the drag leaves carry the `GestureRecognizer` and `OneSequence` bags only.

- Change: Dart's sealed `DragGestureRecognizer` is a trait blanket-implemented for the three axis leaves, whose fields are a private bag on each leaf; the members Dart marks `@protected` are public on it, and `super.handleEvent(event)` reads `DragGestureRecognizer::handle_event(self, app, event)`.
  Reason: language — no inheritance, and a base whose bodies call leaf virtuals cannot hide those bodies behind a narrower visibility than the trait that carries its public members.
  Affect: `use reveal_gestures::DragGestureRecognizer` to reach the drag callbacks and settings on any of the three leaves; do not call the protected members — the arena and the router do.

- Change: the drag recognizer's overridable members live on a `DragLeaf` trait that each axis recognizer implements, and the axis recognizers' bodies are shared functions a leaf outside the crate can call.
  Reason: language — the axis recognizers are subclassed outside this crate, and Rust has no inheritance.
  Affect: a drag subclass in another crate holds the bags and calls the base bodies by name.

## recognizer.rs / tap.rs / long_press.rs / monodrag.rs — GestureRecognizer hierarchy

Pattern: [leaf-inheritance](../../../.cursor/skills/porting-flutter/patterns/leaf-inheritance.md).

- Change: the whole recognizer chain is public — the field bags, the leaf traits and the `super` namespaces of `GestureRecognizer`, `OneSequenceGestureRecognizer`, `PrimaryPointerGestureRecognizer` and `BaseTapGestureRecognizer` — with Dart's `@protected` members public on the namespaces and the library-private touch-slop sentinel public too. The bags stay opaque apart from the gesture settings, Dart's public `gestureSettings`.
  Reason: language — those classes are subclassable from any Dart library, and a leaf written in another crate has to name every piece the base would have inherited.
  Affect: a recognizer written outside this crate is one struct holding the bag of each level in its chain plus its own callbacks, implementing the data trait and the leaf trait of each level and forwarding to the namespace function where Dart writes `super`. `BaseTapGestureRecognizer`'s constructor is `PrimaryPointerData::new` with the press timeout and the unset-slop sentinel; `debug_description` is required, so a leaf that inherited Dart's `'base tap'` writes it itself.

- Change: Dart's `GestureRecognizer` used as a type (a field, a map value) is the erased `AnyGestureRecognizer` — an id plus a static vtable, minted by `as_recognizer()` on every leaf — with `type_id` for Dart's `runtimeType` and `downcast` for `recognizer as T`.
  Reason: language — no inheritance, and an arena id has no fat pointer for a `dyn` trait.
  Affect: store `tap.as_recognizer()` where Dart stores a `GestureRecognizer`; it carries the pointer-adding, `is_pointer_allowed`, `dispose` and `debug_description` calls. Bound a generic by `GestureRecognizerLeaf` where Dart writes `T extends GestureRecognizer`.

- Change: `dispose` frees the arena slot after Dart's body, on the typed handle and the type-erased handle alike.
  Reason: language — Dart leaves the disposed object to the collector; the arena has none.
  Affect: every typed and type-erased handle to the recognizer is stale after `dispose`; calling it twice panics.

## velocity_tracker.rs → velocity_tracker.dart

- Change: `VelocityTracker`'s "time since last sample" is `Instant::now()`, not `GestureBinding.samplingClock.stopwatch()`.
  Reason: language — `SamplingClock` is deferred with resampling; there is no isolate `Stopwatch` tied to FakeAsync.
  Affect: the 40ms "pointer stopped" path follows wall time, not `App::elapse`.

## Deferred

- `PointerEnterEvent` / `PointerExitEvent`. Trigger: `MouseTracker`.
- `NativeHitTestTarget`. Trigger: platform views.
- `debugOwner` / `debugFillProperties` on recognizers. Trigger: diagnostics (F3).
- `_Resampler` / `SamplingClock` / `resamplingEnabled`. Trigger: a host that wants touch resampling.
- `multidrag.dart` (`MultiDragGestureRecognizer` and its `Immediate` / `HorizontalMulti` / `VerticalMulti` / `Delayed` leaves). Trigger: the first widget that drags several pointers independently (`ReorderableListView`).
- Engine `onHitTest`. Trigger: platform views.
