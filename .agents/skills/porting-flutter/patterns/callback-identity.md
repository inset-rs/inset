# Callback identity

Dart compares function values by identity. Tear-offs of the same method on the same object compare equal; two closure literals never do.

Rust closures have no identity. Wrap the callback:

- [`Listener`](../../../crates/inset-foundation/src/change_notifier.rs) for `VoidCallback`
- `PointerRoute` for `typedef PointerRoute = void Function(PointerEvent event)`

`new` equals only its clones (`Rc::ptr_eq`). `handle_method(handle, function_name)` equals any other built from the same handle and function (`TypeId` of `F`, not a fn pointer address — release codegen can merge identical bodies). Pass the function by name: a capturing closure or fn pointer is rejected at compile time (`size_of::<F>() == 0`).

Removal rebuilds the tear-off. Keep the `new` value if the route was a closure.

```
router.add_route(app, pointer, PointerRoute::handle_method(this, Self::handle_event), transform);
router.remove_route(app, pointer, &PointerRoute::handle_method(this, Self::handle_event));
```
