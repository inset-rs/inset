# Handle newtype

Do not `impl FooMethods for Handle<T>` — `Handle` is foreign, so that trait is an extra import for an orphan workaround. Wrap the slot:

```
pub struct Foo(Handle<FooData>);
impl Foo {
    pub fn new(app: &mut App, …) -> Foo { Foo(app.create(FooData { … })) }
    pub fn start(self, app: &mut App) { … }
}
```

Inherent methods take `&mut App`. `AnimationNode` and other arena vtables stay on the data type. Tear-offs use `handle_method(self.0, free_fn)` — `handle_method` takes `Handle<T>`, not the newtype.

See `Ticker`, `AnimationController`, `ProxyAnimation`, `Tween`.
