# Handle receiver

A Dart object with identity is one struct in the `App` arena. Its methods take `self: Handle<Self>` (foundation has `impl<T> Receiver for Handle<T>`, nightly `arbitrary_self_types`) and `&App` / `&mut App`; the body reads `app.get(self)`. Constructors return `Ref<Self>`, which keeps the object; who keeps what is [ownership.md](ownership.md). No `Foo(Handle<FooData>)` newtype, no `FooData`.

```
pub struct Ticker { on_tick: TickerCallback, … }
impl Ticker {
    pub fn new(app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> { app.create(Ticker { … }) }
    pub fn start(self: Handle<Self>, app: &mut App) -> TickerFuture { … }
}
```

Rules:

1. Point access only: `app.get_mut(self)` is a borrow that ends before any call that can re-enter the slot (a callback, a child, `notify_listeners`).
2. A trait whose methods take `self: Handle<Self>` needs `Sized + 'static` as supertraits.
3. Tear-offs take the handle: `Listener::handle_method(self, Self::tick)`. A tear-off keeps nothing, and is skipped once its receiver is freed.
4. Callers of an inherent `self: Handle<Self>` method need `#![feature(arbitrary_self_types)]` in their crate; trait methods resolve without it.
5. A foreign trait with `&self` methods that must hold on the object (it is kept erased: `Rc<dyn Listenable>`, `Rc<dyn GestureBindingOverrides>`) cannot be implemented for `Ref<LocalType>` — orphan rule. The trait's crate provides an object-side twin with `self: Handle<Self>` methods and a blanket impl for `Ref<T>`, so the `Rc` keeps the object; where a holder names the object without keeping it, also for `Handle<T>`: `ListenableObject` → `Listenable for Ref<T>`; `GestureBindingOverridesObject` → `HitTestable` + `GestureBindingOverrides` for both. Implement the twin on the type.
6. Singletons: `App::singleton::<Foo>()` needs `Foo: Default`; `Foo::instance(app) -> Handle<Foo>` fills what `Default` cannot mint on first call. The `App` keeps a singleton.

Erased edges (Dart's abstract class used as a type): trait named after the Flutter class, erased struct `AnyFoo { id: HandleId, vtable: &'static FooVTable }`, table built by `const fn of::<T: Foo>()`, `resolve(id) = Handle::from_id(id)` (free; `App::get` checks staleness), and the object gets its edge from a trait method (`as_animation()`, `as_box()`). A field that keeps one is the alias `FooRef = Retained<AnyFoo>`: the type gives `impl<T: Foo> From<Handle<T>> for AnyFoo`, so a `Ref<T>` becomes a `FooRef` with `erase` / `into_erased`, and `AnyFoo::upgrade(self, app) -> Option<FooRef>`, whose body is `Retained::upgrade(app, self)`. Rendering adds `RenderHandle<T>` because it needs inherent helpers on the handle (`new_box`); nothing else does.

See `Ticker`, `AnimationController`, `Animation` / `AnyAnimation`, `TapGestureRecognizer`, `PipelineOwner`, `MouseTracker`.
