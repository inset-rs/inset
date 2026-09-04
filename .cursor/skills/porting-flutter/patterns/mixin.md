# Mixins and base classes

Rust has no inheritance. A Flutter mixin or base class becomes a trait; its fields become a bag the host holds. Three shapes, by what the Dart type carries.

## Mixin with fields

Opaque `XxxData`, trait named Flutter's `XxxMixin`, host field named after the mixin, accessors `xxx_data` / `xxx_data_mut`. Methods take `self: Handle<Self>` and `App` so a notification can re-enter App. When two mixins both declare a method, write which one applies. Do not leak mixin fields on the host.

```
pub struct AnimationController {
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
    …
}
impl AnimationLazyListenerMixin for AnimationController {
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData { &app.get(self).lazy_listener }
    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData { &mut app.get_mut(self).lazy_listener }
    fn did_start_listening(self: Handle<Self>, app: &mut App) { … }
}
```

Dart's `mixin class ChangeNotifier` is the same shape: `ChangeNotifierData`, field `change_notifier`, accessors `change_notifier_data` / `change_notifier_data_mut`, trait named `ChangeNotifier`. Bag methods that do not need App stay on the data; `notify_listeners` lives on the handle. Implementing the trait also makes the handle `Listenable` (see [handle-receiver.md](handle-receiver.md), rule 5).

## Base class whose subclasses are the leaves

The base class's fields are a bag with an accessor macro; the trait holds the virtuals and the shared bodies. The leaf implements the trait, and where Dart runs the inherited body it calls it by name: `RenderProxyBoxMixin::paint(self, app, context, offset)`. A trait default cannot call `super`, so an override that must run after the base body is a post-hook (`did_attach` / `did_detach`); verify against every Flutter override before choosing pre or post.

```
pub struct RenderPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    padding: EdgeInsetsGeometry,
}
impl RenderObject for RenderPadding { crate::render_object_accessors!(); fn perform_layout(…) { … } }
impl RenderBox for RenderPadding { crate::render_box_accessors!(); }
impl RenderShiftedBox for RenderPadding {}
```

Deep chains (`GestureRecognizer` → `OneSequence` → `PrimaryPointer` → `Tap`) are [leaf-inheritance.md](leaf-inheritance.md): the same bags, with `super` as a namespace fn.

## Stateless mixin or interface

Trait only, no bag: `RenderProxyBoxWithHitTestBehavior`, `Animatable`, `HitTestTarget`. A Dart `implements X` check on an erased object (`target is MouseTrackerAnnotation`) has to be answered by the object's vtable: a defaulted virtual, or a cast slot the type fills (`as_box` / `as_sliver`).

See `listener_helpers.rs`, `change_notifier.rs`, `proxy_box.rs`, `shifted_box.rs`.
