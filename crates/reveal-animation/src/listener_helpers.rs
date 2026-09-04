//! Flutter counterpart: `animation/listener_helpers.dart`.

use reveal_foundation::{App, Handle, HashedObserverList, Listener, ObserverList};

use crate::animation::{AnimationStatus, AnimationStatusListener};

/// Mixin state for [`AnimationLazyListenerMixin`].
///
/// Dart's `AnimationLazyListenerMixin` declares `int _listenerCounter = 0`.
/// A Rust trait holds none, so the host owns this field and the mixin
/// reaches it through [`AnimationLazyListenerMixin::lazy_listener_data_mut`].
#[derive(Default)]
pub struct AnimationLazyListenerData {
    listener_counter: usize,
}

impl AnimationLazyListenerData {
    /// An empty counter — the host is not listening.
    pub fn new() -> AnimationLazyListenerData {
        AnimationLazyListenerData::default()
    }
}

/// A mixin that helps listen to another object only when this object has registered listeners.
///
/// This mixin provides implementations of [`did_register_listener`] and
/// [`did_unregister_listener`], and therefore can be used in conjunction with
/// mixins that require these methods, [`AnimationLocalListenersMixin`] and
/// [`AnimationLocalStatusListenersMixin`].
///
/// [`did_register_listener`]: AnimationLazyListenerMixin::did_register_listener
/// [`did_unregister_listener`]: AnimationLazyListenerMixin::did_unregister_listener
pub trait AnimationLazyListenerMixin: Sized + 'static {
    /// The mixin's field on the host.
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData;

    /// The mixin's field on the host, mutably.
    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData;

    /// Called when the number of listeners changes from zero to one.
    ///
    /// Takes the App because Flutter's implementors reach other objects here:
    /// `ProxyAnimation.didStartListening` registers its own `notifyListeners`
    /// on its parent animation (`animation/animations.dart:227`), which needs
    /// the App to reach the parent and this handle to name the notifier the
    /// parent should call back.
    fn did_start_listening(self: Handle<Self>, app: &mut App);

    /// Called when the number of listeners changes from one to zero.
    fn did_stop_listening(self: Handle<Self>, app: &mut App);

    /// Calls [`did_start_listening`] every time a registration of a listener
    /// causes an empty list of listeners to become non-empty.
    ///
    /// See also:
    ///
    ///  * [`did_unregister_listener`], which may cause the listener list to
    ///    become empty again, and in turn cause this method to call
    ///    [`did_start_listening`] again.
    ///
    /// [`did_start_listening`]: AnimationLazyListenerMixin::did_start_listening
    /// [`did_unregister_listener`]: AnimationLazyListenerMixin::did_unregister_listener
    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        if self.lazy_listener_data(app).listener_counter == 0 {
            self.did_start_listening(app);
        }
        self.lazy_listener_data_mut(app).listener_counter += 1;
    }

    /// Calls [`did_stop_listening`] when an only remaining listener is
    /// unregistered, thus making the list empty.
    ///
    /// See also:
    ///
    ///  * [`did_register_listener`], which causes the listener list to become
    ///    non-empty.
    ///
    /// [`did_stop_listening`]: AnimationLazyListenerMixin::did_stop_listening
    /// [`did_register_listener`]: AnimationLazyListenerMixin::did_register_listener
    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.lazy_listener_data(app).listener_counter >= 1);
        self.lazy_listener_data_mut(app).listener_counter -= 1;
        if self.lazy_listener_data(app).listener_counter == 0 {
            self.did_stop_listening(app);
        }
    }

    /// Whether there are any listeners.
    // `self` is a `Copy` handle and the name is Dart's spec.
    #[allow(clippy::wrong_self_convention)]
    fn is_listening(self: Handle<Self>, app: &App) -> bool {
        self.lazy_listener_data(app).listener_counter > 0
    }
}

/// A mixin that replaces the [`AnimationLazyListenerMixin::did_register_listener`] /
/// [`AnimationLazyListenerMixin::did_unregister_listener`] contract with a dispose
/// contract.
///
/// This mixin provides implementations of [`did_register_listener`] and
/// [`did_unregister_listener`], and therefore can be used in conjunction with
/// mixins that require these methods, [`AnimationLocalListenersMixin`] and
/// [`AnimationLocalStatusListenersMixin`].
///
/// Dart's `AnimationEagerListenerMixin` declares no fields.
///
/// [`did_register_listener`]: AnimationEagerListenerMixin::did_register_listener
/// [`did_unregister_listener`]: AnimationEagerListenerMixin::did_unregister_listener
pub trait AnimationEagerListenerMixin: Sized + 'static {
    /// This implementation ignores listener registrations.
    fn did_register_listener(self: Handle<Self>, _app: &mut App) {}

    /// This implementation ignores listener registrations.
    fn did_unregister_listener(self: Handle<Self>, _app: &mut App) {}

    /// Release the resources used by this object. The object is no longer usable
    /// after this method is called.
    ///
    /// An implementor that overrides this must end by calling
    /// `AnimationEagerListenerMixin::dispose(self: Handle<Self>, app)`, as Dart's `@mustCallSuper`
    /// requires.
    fn dispose(self: Handle<Self>, _app: &mut App) {}
}

/// Mixin state for [`AnimationLocalListenersMixin`].
///
/// Dart's `AnimationLocalListenersMixin` declares
/// `final HashedObserverList<VoidCallback> _listeners`.
#[derive(Default)]
pub struct AnimationLocalListenersData {
    listeners: HashedObserverList<Listener>,
}

impl AnimationLocalListenersData {
    /// An empty listener list.
    pub fn new() -> AnimationLocalListenersData {
        AnimationLocalListenersData::default()
    }

    /// Whether the list has no listeners.
    pub fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }

    /// Whether the list contains `listener`.
    pub fn contains(&self, listener: &Listener) -> bool {
        self.listeners.contains(listener)
    }

    /// The listeners, each once, in the order the first instance of each was
    /// added.
    pub fn to_list(&self) -> Vec<Listener> {
        self.listeners.to_list()
    }
}

/// A mixin that implements the [`add_listener`] / [`remove_listener`] protocol
/// and notifies all the registered listeners when [`notify_listeners`] is
/// called.
///
/// This mixin requires that the implementing type provide methods
/// [`did_register_listener`] and [`did_unregister_listener`]. Implementations
/// of these methods can be obtained by mixing in another mixin from this
/// module, such as [`AnimationLazyListenerMixin`].
///
/// [`add_listener`]: AnimationLocalListenersMixin::add_listener
/// [`remove_listener`]: AnimationLocalListenersMixin::remove_listener
/// [`notify_listeners`]: AnimationLocalListenersMixin::notify_listeners
/// [`did_register_listener`]: AnimationLocalListenersMixin::did_register_listener
/// [`did_unregister_listener`]: AnimationLocalListenersMixin::did_unregister_listener
pub trait AnimationLocalListenersMixin: Sized + 'static {
    /// The mixin's field on the host.
    fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData;

    /// The mixin's field on the host, mutably.
    fn local_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalListenersData;

    /// Called immediately before a listener is added via [`add_listener`].
    ///
    /// At the time this method is called the registered listener is not yet
    /// notified by [`notify_listeners`].
    ///
    /// [`add_listener`]: AnimationLocalListenersMixin::add_listener
    /// [`notify_listeners`]: AnimationLocalListenersMixin::notify_listeners
    fn did_register_listener(self: Handle<Self>, app: &mut App);

    /// Called immediately after a listener is removed via [`remove_listener`].
    ///
    /// At the time this method is called the removed listener is no longer
    /// notified by [`notify_listeners`].
    ///
    /// [`remove_listener`]: AnimationLocalListenersMixin::remove_listener
    /// [`notify_listeners`]: AnimationLocalListenersMixin::notify_listeners
    fn did_unregister_listener(self: Handle<Self>, app: &mut App);

    /// Calls the listener every time the value of the animation changes.
    ///
    /// Listeners can be removed with [`remove_listener`].
    ///
    /// [`remove_listener`]: AnimationLocalListenersMixin::remove_listener
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        self.did_register_listener(app);
        self.local_listeners_data_mut(app).listeners.add(listener);
    }

    /// Stop calling the listener every time the value of the animation changes.
    ///
    /// Listeners can be added with [`add_listener`].
    ///
    /// [`add_listener`]: AnimationLocalListenersMixin::add_listener
    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        let removed = self
            .local_listeners_data_mut(app)
            .listeners
            .remove(listener);
        if removed {
            self.did_unregister_listener(app);
        }
    }

    /// Removes all listeners added with [`add_listener`].
    ///
    /// This method is typically called from the `dispose` method of the type
    /// using this mixin if the type also uses [`AnimationEagerListenerMixin`].
    ///
    /// Calling this method will not trigger
    /// [`did_unregister_listener`](AnimationLocalListenersMixin::did_unregister_listener).
    ///
    /// [`add_listener`]: AnimationLocalListenersMixin::add_listener
    fn clear_listeners(self: Handle<Self>, app: &mut App) {
        self.local_listeners_data_mut(app).listeners.clear();
    }

    /// Calls all the listeners.
    ///
    /// If listeners are added or removed during this function, the modifications
    /// will not change which listeners are called during this iteration.
    fn notify_listeners(self: Handle<Self>, app: &mut App) {
        let local_listeners = self.local_listeners_data(app).to_list();
        for listener in local_listeners {
            // Dart wraps this in a try/catch that reports the exception and
            // carries on to the next listener; a panicking listener here
            // unwinds instead. Unlike `ChangeNotifier.notifyListeners`, nothing
            // runs after this loop, so the catch has no second job to lose.
            // The slot is re-read here, and `to_list` above already cloned
            // every listener out of it, so nothing of `app` is borrowed while
            // one runs — which is what lets a listener reach its own host.
            if self.local_listeners_data(app).contains(&listener) {
                listener.call(app);
            }
        }
    }
}

/// Mixin state for [`AnimationLocalStatusListenersMixin`].
///
/// Dart's `AnimationLocalStatusListenersMixin` declares
/// `final ObserverList<AnimationStatusListener> _statusListeners`.
#[derive(Default)]
pub struct AnimationLocalStatusListenersData {
    status_listeners: ObserverList<AnimationStatusListener>,
}

impl AnimationLocalStatusListenersData {
    /// An empty status-listener list.
    pub fn new() -> AnimationLocalStatusListenersData {
        AnimationLocalStatusListenersData::default()
    }

    /// Whether the list has no listeners.
    pub fn is_empty(&self) -> bool {
        self.status_listeners.is_empty()
    }

    /// Whether the list contains `listener`.
    pub fn contains(&mut self, listener: &AnimationStatusListener) -> bool {
        self.status_listeners.contains(listener)
    }

    /// The listeners, in the order they were added.
    pub fn to_list(&self) -> Vec<AnimationStatusListener> {
        self.status_listeners.to_list()
    }
}

/// A mixin that implements the [`add_status_listener`] / [`remove_status_listener`]
/// protocol and notifies all the registered listeners when
/// [`notify_status_listeners`] is called.
///
/// This mixin requires that the implementing type provide methods
/// [`did_register_listener`] and [`did_unregister_listener`]. Implementations
/// of these methods can be obtained by mixing in another mixin from this
/// module, such as [`AnimationLazyListenerMixin`].
///
/// [`add_status_listener`]: AnimationLocalStatusListenersMixin::add_status_listener
/// [`remove_status_listener`]: AnimationLocalStatusListenersMixin::remove_status_listener
/// [`notify_status_listeners`]: AnimationLocalStatusListenersMixin::notify_status_listeners
/// [`did_register_listener`]: AnimationLocalStatusListenersMixin::did_register_listener
/// [`did_unregister_listener`]: AnimationLocalStatusListenersMixin::did_unregister_listener
pub trait AnimationLocalStatusListenersMixin: Sized + 'static {
    /// The mixin's field on the host.
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData;

    /// The mixin's field on the host, mutably.
    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData;

    /// Called immediately before a status listener is added via
    /// [`add_status_listener`].
    ///
    /// At the time this method is called the registered listener is not yet
    /// notified by [`notify_status_listeners`].
    ///
    /// [`add_status_listener`]: AnimationLocalStatusListenersMixin::add_status_listener
    /// [`notify_status_listeners`]: AnimationLocalStatusListenersMixin::notify_status_listeners
    fn did_register_listener(self: Handle<Self>, app: &mut App);

    /// Called immediately after a status listener is removed via
    /// [`remove_status_listener`].
    ///
    /// At the time this method is called the removed listener is no longer
    /// notified by [`notify_status_listeners`].
    ///
    /// [`remove_status_listener`]: AnimationLocalStatusListenersMixin::remove_status_listener
    /// [`notify_status_listeners`]: AnimationLocalStatusListenersMixin::notify_status_listeners
    fn did_unregister_listener(self: Handle<Self>, app: &mut App);

    /// Calls listener every time the status of the animation changes.
    ///
    /// Listeners can be removed with [`remove_status_listener`].
    ///
    /// [`remove_status_listener`]: AnimationLocalStatusListenersMixin::remove_status_listener
    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        self.did_register_listener(app);
        self.local_status_listeners_data_mut(app)
            .status_listeners
            .add(listener);
    }

    /// Stops calling the listener every time the status of the animation changes.
    ///
    /// Listeners can be added with [`add_status_listener`].
    ///
    /// [`add_status_listener`]: AnimationLocalStatusListenersMixin::add_status_listener
    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        let removed = self
            .local_status_listeners_data_mut(app)
            .status_listeners
            .remove(listener);
        if removed {
            self.did_unregister_listener(app);
        }
    }

    /// Removes all listeners added with [`add_status_listener`].
    ///
    /// This method is typically called from the `dispose` method of the type
    /// using this mixin if the type also uses [`AnimationEagerListenerMixin`].
    ///
    /// Calling this method will not trigger
    /// [`did_unregister_listener`](AnimationLocalStatusListenersMixin::did_unregister_listener).
    ///
    /// [`add_status_listener`]: AnimationLocalStatusListenersMixin::add_status_listener
    fn clear_status_listeners(self: Handle<Self>, app: &mut App) {
        self.local_status_listeners_data_mut(app)
            .status_listeners
            .clear();
    }

    /// Calls all the status listeners.
    ///
    /// If listeners are added or removed during this function, the modifications
    /// will not change which listeners are called during this iteration.
    fn notify_status_listeners(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        let local_listeners = self.local_status_listeners_data(app).to_list();
        for listener in local_listeners {
            // Dart wraps this in a try/catch that reports the exception and
            // carries on to the next listener; a panicking listener here
            // unwinds instead. Nothing runs after this loop, so the catch has
            // no second job to lose.
            if self
                .local_status_listeners_data_mut(app)
                .contains(&listener)
            {
                listener.call(status, app);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    type Log = Rc<RefCell<Vec<String>>>;

    /// A host applying Lazy + LocalListeners, as `ProxyAnimation` and
    /// `CompoundAnimation` do. One field per mixin, exactly as in Dart.
    #[derive(Default)]
    struct LazyHost {
        lazy_listener: AnimationLazyListenerData,
        local_listeners: AnimationLocalListenersData,
        log: Log,
    }

    impl AnimationLazyListenerMixin for LazyHost {
        fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
            &app.get(self).lazy_listener
        }

        fn lazy_listener_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AnimationLazyListenerData {
            &mut app.get_mut(self).lazy_listener
        }

        fn did_start_listening(self: Handle<Self>, app: &mut App) {
            app.get(self).log.borrow_mut().push("start".to_string());
        }

        fn did_stop_listening(self: Handle<Self>, app: &mut App) {
            app.get(self).log.borrow_mut().push("stop".to_string());
        }
    }

    // Dart resolves `didRegisterListener` for a class applying both mixins by
    // linearization; here the choice is written out.
    impl AnimationLocalListenersMixin for LazyHost {
        fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
            &app.get(self).local_listeners
        }

        fn local_listeners_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AnimationLocalListenersData {
            &mut app.get_mut(self).local_listeners
        }

        fn did_register_listener(self: Handle<Self>, app: &mut App) {
            AnimationLazyListenerMixin::did_register_listener(self, app);
        }

        fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
            AnimationLazyListenerMixin::did_unregister_listener(self, app);
        }
    }

    /// A host applying Eager + LocalListeners, as `AnimationController` and
    /// `TrainHoppingAnimation` do.
    #[derive(Default)]
    struct EagerHost {
        local_listeners: AnimationLocalListenersData,
        disposed: bool,
    }

    impl AnimationEagerListenerMixin for EagerHost {
        fn dispose(self: Handle<Self>, app: &mut App) {
            self.clear_listeners(app);
            app.get_mut(self).disposed = true;
        }
    }

    impl AnimationLocalListenersMixin for EagerHost {
        fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
            &app.get(self).local_listeners
        }

        fn local_listeners_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AnimationLocalListenersData {
            &mut app.get_mut(self).local_listeners
        }

        fn did_register_listener(self: Handle<Self>, app: &mut App) {
            AnimationEagerListenerMixin::did_register_listener(self, app);
        }

        fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
            AnimationEagerListenerMixin::did_unregister_listener(self, app);
        }
    }

    /// A host applying Lazy + LocalStatusListeners and *not* LocalListeners, as
    /// `ReverseAnimation` does (`animations.dart:273`). The mixins are
    /// independent, so this combination is two impls rather than a special case.
    #[derive(Default)]
    struct ReverseHost {
        lazy_listener: AnimationLazyListenerData,
        local_status_listeners: AnimationLocalStatusListenersData,
        log: Log,
    }

    impl AnimationLazyListenerMixin for ReverseHost {
        fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
            &app.get(self).lazy_listener
        }

        fn lazy_listener_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AnimationLazyListenerData {
            &mut app.get_mut(self).lazy_listener
        }

        fn did_start_listening(self: Handle<Self>, app: &mut App) {
            app.get(self).log.borrow_mut().push("start".to_string());
        }

        fn did_stop_listening(self: Handle<Self>, app: &mut App) {
            app.get(self).log.borrow_mut().push("stop".to_string());
        }
    }

    impl AnimationLocalStatusListenersMixin for ReverseHost {
        fn local_status_listeners_data(
            self: Handle<Self>,
            app: &App,
        ) -> &AnimationLocalStatusListenersData {
            &app.get(self).local_status_listeners
        }

        fn local_status_listeners_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AnimationLocalStatusListenersData {
            &mut app.get_mut(self).local_status_listeners
        }

        fn did_register_listener(self: Handle<Self>, app: &mut App) {
            AnimationLazyListenerMixin::did_register_listener(self, app);
        }

        fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
            AnimationLazyListenerMixin::did_unregister_listener(self, app);
        }
    }

    /// A host whose start/stop hooks reach *another* host through the App,
    /// which is the shape `ProxyAnimation` uses to forward its parent's
    /// notifications (`animation/animations.dart:227`, `:235`). No
    /// `AnyAnimation<T>` here — only the hook signature is being exercised.
    #[derive(Default)]
    struct ProxyLikeHost {
        lazy_listener: AnimationLazyListenerData,
        local_listeners: AnimationLocalListenersData,
        parent: Option<Handle<EagerHost>>,
        /// The listener registered on the parent, kept so it can be removed.
        on_parent: Option<Listener>,
        log: Log,
    }

    impl AnimationLazyListenerMixin for ProxyLikeHost {
        fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
            &app.get(self).lazy_listener
        }

        fn lazy_listener_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AnimationLazyListenerData {
            &mut app.get_mut(self).lazy_listener
        }

        fn did_start_listening(self: Handle<Self>, app: &mut App) {
            let Some(parent) = app.get(self).parent else {
                return;
            };
            // Dart passes the tear-off `notifyListeners`; the counterpart is a
            // listener that names this host and dispatches back through it.
            let forward = Listener::new(move |app: &mut App| {
                self.notify_listeners(app);
            });
            app.get_mut(self).on_parent = Some(forward.clone());
            parent.add_listener(app, forward);
        }

        fn did_stop_listening(self: Handle<Self>, app: &mut App) {
            let Some(parent) = app.get(self).parent else {
                return;
            };
            let Some(forward) = app.get_mut(self).on_parent.take() else {
                return;
            };
            parent.remove_listener(app, &forward);
        }
    }

    impl AnimationLocalListenersMixin for ProxyLikeHost {
        fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
            &app.get(self).local_listeners
        }

        fn local_listeners_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut AnimationLocalListenersData {
            &mut app.get_mut(self).local_listeners
        }

        fn did_register_listener(self: Handle<Self>, app: &mut App) {
            AnimationLazyListenerMixin::did_register_listener(self, app);
        }

        fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
            AnimationLazyListenerMixin::did_unregister_listener(self, app);
        }
    }

    fn recording_status(log: &Log, name: &'static str) -> AnimationStatusListener {
        let log = Rc::clone(log);
        AnimationStatusListener::new(move |status, _app| {
            log.borrow_mut().push(format!("{name}:{status:?}"));
        })
    }

    fn recording(log: &Log, name: &'static str) -> Listener {
        let log = Rc::clone(log);
        Listener::new(move |_app| log.borrow_mut().push(name.to_string()))
    }

    fn fired(log: &Log) -> Vec<String> {
        log.borrow().clone()
    }

    #[test]
    fn a_lazy_host_starts_listening_on_the_first_listener_and_stops_on_the_last() {
        let mut app = App::new();
        let host = app.create(LazyHost::default());
        let log = Rc::clone(&app.get(host).log);

        let first = recording(&log, "first");
        let second = recording(&log, "second");

        host.add_listener(&mut app, first.clone());
        assert_eq!(fired(&log), ["start"], "the empty list became non-empty");
        assert!(host.is_listening(&app));

        host.add_listener(&mut app, second.clone());
        assert_eq!(fired(&log), ["start"], "no second start");

        host.remove_listener(&mut app, &first);
        assert_eq!(fired(&log), ["start"], "one listener remains");

        host.remove_listener(&mut app, &second);
        assert_eq!(fired(&log), ["start", "stop"], "the list became empty");
        assert!(!host.is_listening(&app));
    }

    #[test]
    fn removing_an_unregistered_listener_does_not_reach_did_unregister_listener() {
        let mut app = App::new();
        let host = app.create(LazyHost::default());
        let log = Rc::clone(&app.get(host).log);

        let registered = recording(&log, "registered");
        host.add_listener(&mut app, registered);

        let stranger = recording(&log, "stranger");
        host.remove_listener(&mut app, &stranger);
        assert_eq!(fired(&log), ["start"], "the counter did not move");
        assert!(host.is_listening(&app));
    }

    #[test]
    fn a_lazy_host_starts_again_after_going_empty() {
        let mut app = App::new();
        let host = app.create(LazyHost::default());
        let log = Rc::clone(&app.get(host).log);

        let listener = recording(&log, "listener");
        host.add_listener(&mut app, listener.clone());
        host.remove_listener(&mut app, &listener);
        host.add_listener(&mut app, listener.clone());

        assert_eq!(fired(&log), ["start", "stop", "start"]);
    }

    #[test]
    fn an_eager_host_never_starts_or_stops_but_disposes() {
        let mut app = App::new();
        let host = app.create(EagerHost::default());
        let log: Log = Log::default();

        host.add_listener(&mut app, recording(&log, "listener"));
        assert!(!host.local_listeners_data(&app).is_empty());

        AnimationEagerListenerMixin::dispose(host, &mut app);
        assert!(app.get(host).disposed);
        assert!(
            host.local_listeners_data(&app).is_empty(),
            "dispose cleared the listeners"
        );
    }

    #[test]
    fn notify_listeners_calls_every_registered_listener() {
        let mut app = App::new();
        let host = app.create(EagerHost::default());
        let log: Log = Log::default();

        host.add_listener(&mut app, recording(&log, "one"));
        host.add_listener(&mut app, recording(&log, "two"));
        host.notify_listeners(&mut app);

        assert_eq!(fired(&log), ["one", "two"]);
    }

    /// Pins the `contains` re-check in
    /// `AnimationLocalListenersMixin.notifyListeners`, on the real path.
    ///
    /// Flutter's case is `test/animation/iteration_patterns_test.dart:36`:
    /// `listener2` removes `listener3` before its turn comes, and `listener3`
    /// is never called. Batch 3 could only reach this through a test double
    /// answering two reads differently, because a `Listener` could not reach
    /// its host; addressing the host through the App is what makes the Dart
    /// scenario itself expressible. Deleting the guard makes this fail.
    #[test]
    fn a_listener_removed_mid_dispatch_is_not_called() {
        let mut app = App::new();
        let host = app.create(EagerHost::default());
        let log: Log = Log::default();

        let doomed = recording(&log, "doomed");
        let remover = {
            let log = Rc::clone(&log);
            let doomed = doomed.clone();
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("remover".to_string());
                host.remove_listener(app, &doomed);
            })
        };

        host.add_listener(&mut app, remover);
        host.add_listener(&mut app, doomed);

        host.notify_listeners(&mut app);

        assert_eq!(
            fired(&log),
            ["remover"],
            "the guard must re-check the live list, not trust the snapshot"
        );
    }

    #[test]
    fn a_lazy_status_only_host_starts_and_stops_on_its_status_listeners() {
        let mut app = App::new();
        let host = app.create(ReverseHost::default());
        let log = Rc::clone(&app.get(host).log);

        let listener = recording_status(&log, "status");
        host.add_status_listener(&mut app, listener.clone());
        assert_eq!(fired(&log), ["start"]);
        assert!(host.is_listening(&app));

        host.notify_status_listeners(&mut app, AnimationStatus::Forward);
        assert_eq!(fired(&log), ["start", "status:Forward"]);

        host.remove_status_listener(&mut app, &listener);
        assert_eq!(
            fired(&log),
            ["start", "status:Forward", "stop"],
            "the last status listener went away"
        );
        assert!(!host.is_listening(&app));
    }

    /// The same pin for `notifyStatusListeners`, whose Flutter case is
    /// `iteration_patterns_test.dart:74`.
    #[test]
    fn a_status_listener_removed_mid_dispatch_is_not_called() {
        let mut app = App::new();
        let host = app.create(ReverseHost::default());
        let log = Rc::clone(&app.get(host).log);

        let doomed = recording_status(&log, "doomed");
        let remover = {
            let log = Rc::clone(&log);
            let doomed = doomed.clone();
            AnimationStatusListener::new(move |_status, app: &mut App| {
                log.borrow_mut().push("remover".to_string());
                host.remove_status_listener(app, &doomed);
            })
        };

        host.add_status_listener(&mut app, remover);
        host.add_status_listener(&mut app, doomed);
        log.borrow_mut().clear();

        host.notify_status_listeners(&mut app, AnimationStatus::Completed);

        assert_eq!(
            fired(&log),
            ["remover"],
            "the guard must re-check the live list, not trust the snapshot"
        );
    }

    /// `didStartListening` reaching another object through the App is the whole
    /// reason the hooks take the App: `ProxyAnimation.didStartListening`
    /// registers its own `notifyListeners` on its parent
    /// (`animation/animations.dart:227`). This host does the same shape without
    /// `AnyAnimation<T>` — it registers a listener on a *second* host when its own
    /// first listener arrives, and removes it when its last one goes.
    #[test]
    fn a_start_hook_can_register_on_another_host_through_the_app() {
        let mut app = App::new();
        let parent = app.create(EagerHost::default());
        let proxy = app.create(ProxyLikeHost::default());
        let log = Rc::clone(&app.get(proxy).log);
        app.get_mut(proxy).parent = Some(parent);

        // No listeners yet, so the proxy has not subscribed to its parent.
        assert!(parent.local_listeners_data(&app).is_empty());

        proxy.add_listener(&mut app, recording(&log, "own"));
        assert!(
            !parent.local_listeners_data(&app).is_empty(),
            "the start hook subscribed to the parent"
        );

        // A parent notification is forwarded to the proxy's own listeners.
        parent.notify_listeners(&mut app);
        assert_eq!(fired(&log), ["own"]);

        let own = proxy.local_listeners_data(&app).to_list()[0].clone();
        proxy.remove_listener(&mut app, &own);
        assert!(
            parent.local_listeners_data(&app).is_empty(),
            "the stop hook unsubscribed"
        );
    }

    #[test]
    fn clear_listeners_does_not_reach_did_unregister_listener() {
        let mut app = App::new();
        let host = app.create(LazyHost::default());
        let log = Rc::clone(&app.get(host).log);

        host.add_listener(&mut app, recording(&log, "listener"));
        host.clear_listeners(&mut app);

        assert_eq!(fired(&log), ["start"], "no stop, as Dart documents");
        assert!(
            host.is_listening(&app),
            "the counter still counts the cleared listener"
        );
    }
}
