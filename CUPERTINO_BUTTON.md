# CupertinoButton path

Goal: a CupertinoButton that presses (gesture) and fades (opacity animation).

Spec: `/Users/mac/code/flutter/packages/flutter/lib/src/cupertino/button.dart`. Crate order: dart:ui (`reveal-embedder`) → foundation → scheduler / painting / physics → gestures → rendering → widgets → cupertino. Accessibility skipped (do not stub `SemanticsBinding`). No `reveal-widgets`, `reveal-cupertino`, or `reveal-services` crate exists.

## Already in reveal-rs

**reveal-embedder** (dart:ui subset) — `Offset`/`Size`/`Rect`/`RRect`/`RSuperellipse`/`Radius`, `Color`, `Clip`, `Shadow`, `Matrix4`, `Canvas`/`Paint`/`Path`/`Picture` (valo), `TargetPlatform`, `Brightness`, `Platform`/`View`/`EmbedderClient`, `ViewPadding`, `ViewConstraints`, `GestureSettings`, `PointerData`/`PointerDataPacket`. `ui.TextStyle` is valo `TextStyle` (no encode). `FontWeight` / `FontStyle` / `TextDecoration` / `TextAlign` / `TextBaseline` / `TextDirection` / `FontFeature` / `FontVariation` are in.

**reveal-foundation** — `App`/`Handle`, `ChangeNotifier`/`Listenable`/`ValueNotifier`, `kDebugMode`/`kIsWeb`, `ObserverList`, `Key`/`LocalKey`/`UniqueKey`/`ValueKey`, `ValueChanged`, `Timer`. Re-exports `TargetPlatform`. No diagnostics, no `BindingBase`, no `GlobalKey`.

**reveal-scheduler** — `SchedulerBinding` (App singleton), `Ticker`/`TickerFuture`/`TickerProvider`, frame callbacks. Missing `debug.dart` (not on the button path).

**reveal-shell** — `Shell` owns `App` and is the `EmbedderClient`. Frames → `SchedulerBinding`. Pointer packets → `GestureBinding`. View hooks empty until `RendererBinding`.

**reveal-animation** — `Animation`/`AnimationController`/`Tween`/`CurveTween`/`Curves`. `SingleTickerProviderStateMixin` is widgets, not here. `SemanticsBinding.disableAnimations` skipped.

**reveal-painting** — alignment, edge insets, border radius, `BorderSide`/`ShapeBorder`, box/shape decorations (solid color only), `RoundedSuperellipseBorder`, `BoxShadow`, `HSLColor`/`HSVColor`, `ClipContext`, `TextScaler`, painting `TextStyle`. Re-exports dart:ui text enums.

**reveal-physics** — simulations used by `AnimationController`; not required for tap.

**reveal-gestures** — `GestureBinding`, arena, router, team, `TapGestureRecognizer` / `LongPressGestureRecognizer` (leaf Handle; superclass bags + `super` namespaces), tap / long-press details, `Velocity` / `VelocityTracker`.

**reveal-rendering** — `BoxConstraints`, `Constraints`, `HitTestBehavior`, rendering debug flags, `RenderObject` / `RenderBox` / `RenderSliver`, `RenderHandle<T>` + erased `AnyRenderObject` / `AnyRenderBox` / `AnyRenderSliver`, `PipelineOwner` dirty-layout flush, `RenderPadding`, `RenderConstrainedBox`. No `PaintingContext`, layers, or `flushPaint`.

**reveal-embedder-winit** — host window + `View::present`. Mouse down/move/up/hover and wheel → `EmbedderClient::pointer_data_packet`.

## Ordered work

### Gestures (`reveal-gestures`)

| id | Flutter | status | depends | note |
|---|---|---|---|---|
| G1 | `hit_test.dart`, `constants.dart`, `gesture_settings.dart`, `gesture_details.dart` | done | E2, F1 | |
| G2 | `events.dart`, `converter.dart` | done | G1, E2 | |
| G3 | `arena.dart`, `pointer_router.dart`, `recognizer.dart` | done | G2, G6 | |
| G4 | `tap.dart` (`TapGestureRecognizer`) | done | G3 | **The press.** |
| G5 | `long_press.dart` recognizer | done | G3 | Second `PrimaryPointer` leaf. Only if `onLongPress != null`. |
| G6 | `binding.dart` `GestureBinding` | partial | G3, E3 | Resampling / `PointerSignalResolver` deferred. |

### Rendering (`reveal-rendering`)

| id | Flutter | status | depends | note |
|---|---|---|---|---|
| R1 | `object.dart` (`RenderObject`, `PaintingContext`, `PipelineOwner`) + `layer.dart` (`OpacityLayer`) | partial | G1, painting | **Layout/tree in** (`RenderHandle<T>` / `AnyRenderObject`, `PipelineOwner.flushLayout`). No `PaintingContext` / layers / paint flush. Skip semantics APIs. |
| R2 | `box.dart` (`RenderBox`, `BoxConstraints`, `globalToLocal`, `paintBounds`) | partial | R1 | **`BoxConstraints` and the box `layout` wrapper are in.** `globalToLocal` / `BoxHitTestResult` / `_DebugSize` wait. |
| R3 | `proxy_box.dart` slice | partial | R2 | `HitTestBehavior` and `RenderConstrainedBox` layout are in. Fade is `RenderAnimatedOpacity` + `OpacityLayer`. |
| R4 | `shifted_box.dart` (`RenderPadding`, `RenderPositionedBox`) | partial | R2 | **`RenderPadding` layout is in.** `RenderPositionedBox` / paint wait. |
| R5 | `view.dart` `RenderView` + `binding.dart` `RendererBinding` | blocked | R1, G6 | Skip `SemanticsBinding`. `RenderView` is not a box and overrides the `constraints` getter — ask before adding that ops slot. |
| R6 | `mouse_tracker.dart` | next | R3 | |

### Widgets / Cupertino

Unchanged: W1–W10 and C1–C4 still wait on the tree. `CupertinoDynamicColor implements Color` remains a language wall.

## Known walls

1. **`CupertinoDynamicColor implements Color` (language)** — `Color` is a Copy struct stored by value. Ask when C2 is reached.
2. **`RendererBinding` / `WidgetsBinding` mix in `SemanticsBinding`** — skip those members; do not stub the binding.
3. **Diagnostics** — deferred as strings/`debug_assert!` until `diagnostics.dart`.
4. **Layout re-entrancy** — `Handle` point access only. Leasing a render object for a pass is the shaft-rs-next bug.
5. **`GestureRecognizer` inheritance (language)** — one Handle on the leaf; superclasses are field bags; `super` is a namespace fn. See `.cursor/skills/porting-flutter/patterns/leaf-inheritance.md`.
6. **`RenderObject` (language)** — `RenderHandle<T>` + erased `AnyRenderObject` (id + `&'static` vtable, no lease); protocol traits own the edges and the tree methods. Ask before encoding `PaintingContext` / layers.
7. **`layout` / `markNeedsLayout` / `constraints` as virtuals** — not on the ops table. OverlayPortal overrides `layout`; `RenderView` overrides `constraints`. Ask before adding those slots.
