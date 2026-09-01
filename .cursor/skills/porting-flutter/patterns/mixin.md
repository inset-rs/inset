# Mixin with fields

Opaque `XxxData`, trait named Flutter's `XxxMixin`, host field named after the mixin, accessors `xxx_data` / `xxx_data_mut`. Methods live on the handle or newtype (`Copy` + `&mut App`) so notify can re-enter App. Stateless mixin: trait only. When two mixins both declare a method, write which one applies. Do not leak mixin fields on the host.

```
struct Host {
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
}
impl AnimationLazyListenerMixin for Handle<Host> {
    fn lazy_listener_data(self, app: &App) -> &AnimationLazyListenerData { &app.get(self).lazy_listener }
    fn lazy_listener_data_mut(self, app: &mut App) -> &mut AnimationLazyListenerData { &mut app.get_mut(self).lazy_listener }
    fn did_start_listening(self, app: &mut App) { … }
    fn did_stop_listening(self, app: &mut App) { … }
}
```

Dart's `mixin class ChangeNotifier` is the same shape: [`ChangeNotifierData`], field `change_notifier`, accessors `change_notifier_data` / `change_notifier_data_mut`. The mixin trait is named `ChangeNotifier` (Flutter has no `ChangeNotifierMixin`). Bag methods that do not need App stay on the data; `notify_listeners` lives on the handle.

See `listener_helpers.rs`, `change_notifier.rs`.
