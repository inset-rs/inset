# Handle receiver

A Dart object with identity is one struct in the `App` arena. Its methods take `self: Handle<Self>` (foundation has `impl<T> Receiver for Handle<T>`, nightly `arbitrary_self_types`) and `&App` / `&mut App`; the body reads `app.get(self)`. Constructors return `Handle<Self>`. No `Foo(Handle<FooData>)` newtype, no `FooData`.

```
pub struct Ticker { on_tick: TickerCallback, … }
impl Ticker {
    pub fn new(app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> { app.create(Ticker { … }) }
    pub fn start(self: Handle<Self>, app: &mut App) -> Handle<TickerFuture> { … }
}
```

Rules:

1. Point access only: `app.get_mut(self)` is a borrow that ends before any call that can re-enter the slot (a callback, a child, `notify_listeners`).
2. A trait whose methods take `self: Handle<Self>` needs `Sized + 'static` as supertraits.
3. Tear-offs take the handle: `Listener::handle_method(self, Self::tick)`.
4. Callers of an inherent `self: Handle<Self>` method need `#![feature(arbitrary_self_types)]` in their crate; trait methods resolve without it.
5. A foreign trait with `&self` methods that must hold on the handle (it is kept erased: `Rc<dyn Listenable>`, `Rc<dyn GestureBindingOverrides>`) cannot be implemented for `Handle<LocalType>` — orphan rule. The trait's crate provides an object-side twin with `self: Handle<Self>` methods and a blanket impl: `ListenableObject` → `impl<T: ListenableObject> Listenable for Handle<T>`; `GestureBindingOverridesObject` → `HitTestable` + `GestureBindingOverrides`. Implement the twin on the type.
6. Singletons: `App::singleton::<Foo>()` needs `Foo: Default`; `Foo::instance(app) -> Handle<Foo>` fills what `Default` cannot mint on first call.

Erased edges (Dart's abstract class used as a type): trait named after the Flutter class, erased struct `AnyFoo { id: HandleId, vtable: &'static FooVTable }`, table built by `const fn of::<T: Foo>()`, `resolve(id) = Handle::from_id(id)` (free; `App::get` checks staleness), and the object gets its edge from a trait method (`as_animation()`, `as_box()`). Rendering adds `RenderHandle<T>` because it needs inherent helpers on the handle (`new_box`); nothing else does.

See `Ticker`, `AnimationController`, `Animation` / `AnyAnimation`, `TapGestureRecognizer`, `PipelineOwner`, `MouseTracker`.
