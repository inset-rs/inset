# Leaf inheritance

One Handle per Dart object (the **leaf**). Each superclass is a field bag on that slot. `super` is an associated fn on a namespace, called inside the body at Dart's position.

```
pub struct TapGestureRecognizer(Handle<TapGestureData>);

struct TapGestureData {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    primary: PrimaryPointerData,
    base_tap: BaseTapData,
    on_tap: Option<…>,
}
```

Invariants:

1. `super` stays inside the body at Dart's position. Skipping a level drops behavior.
2. Virtuals from a superclass body call the **leaf**. A second leaf must not hard-code a sibling (`BaseTap` inside `PrimaryPointer`). Namespace fns are generic over [`RecognizerLeaf`](../../../crates/reveal-gestures/src/recognizer.rs); the leaf implements the virtuals (defaults match the next superclass).
3. One Handle per Dart object. Never put a bag on its own Handle. A `PrimaryPointer` leaf that is not a tap has no `BaseTap` bag.

The trait is on the newtype, not `impl Methods for Handle<T>` (see [handle-newtype.md](handle-newtype.md)). Tear-offs take the slot: `PointerRoute::handle_method(self.0, …)`, `Listener::handle_method(self.0, …)`. Each leaf's `handle_event_route` / `deadline_fired` is a distinct fn item so `TypeId` matches on remove.

See `TapGestureRecognizer`, `LongPressGestureRecognizer`.
