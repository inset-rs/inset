# Leaf inheritance

One Handle per Dart object (the **leaf**). Each superclass is a field bag on that struct. `super` is an associated fn on a namespace, called inside the body at Dart's position.

```
pub struct TapGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    primary: PrimaryPointerData,
    base_tap: BaseTapData,
    on_tap: Option<…>,
}
```

Invariants:

1. `super` stays inside the body at Dart's position. Skipping a level drops behavior.
2. Virtuals from a superclass body call the **leaf**. A second leaf must not hard-code a sibling (`BaseTap` inside `PrimaryPointer`). Namespace fns are generic over [`RecognizerLeaf`](../../../crates/inset-gestures/src/recognizer.rs) and take `this: Handle<R>`; the leaf implements the virtuals with `self: Handle<Self>` (defaults match the next superclass).
3. One Handle per Dart object. Never put a bag on its own Handle. A `PrimaryPointer` leaf that is not a tap has no `BaseTap` bag.

The leaf is a plain arena struct (see [handle-receiver.md](handle-receiver.md)). What Dart inherits as an interface (`GestureRecognizer extends GestureArenaMember`) is one blanket impl over the leaf trait: `impl<R: RecognizerLeaf> GestureArenaMember for Handle<R>`. Tear-offs take the handle: `PointerRoute::handle_method(self, Self::handle_event_route)`. Each leaf's `handle_event_route` / `deadline_fired` is a distinct fn item so `TypeId` matches on remove.

See `TapGestureRecognizer`, `LongPressGestureRecognizer`.
