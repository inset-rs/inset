# reveal-gestures/src
Flutter home: packages/flutter/lib/src/gestures
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- constants.rs → constants.dart
- gesture_settings.rs → gesture_settings.dart

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

## Deferred

- `PointerEnterEvent` / `PointerExitEvent`. Trigger: `MouseTracker`.
