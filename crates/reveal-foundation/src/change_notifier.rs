//! Flutter counterpart: `foundation/change_notifier.dart`.

use std::any::TypeId;
use std::cell::Cell;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::app::{App, Handle, HandleId};

/// A closure registered with a [`Listenable`].
///
/// Flutter's counterpart is `VoidCallback`. Receives [`App`] because a Rust
/// listener cannot own what it mutates.
///
/// [`Listenable::remove_listener`] matches identity: [`Listener::new`] equals
/// only its clones; [`Listener::handle_method`] equals any other built from the
/// same handle and function.
#[derive(Clone)]
pub struct Listener {
    callback: Rc<dyn Fn(&mut App)>,
    identity: ListenerIdentity,
}

/// Equality key. Closures compare by allocation; handle-methods by
/// `(handle, TypeId of F)`. TypeId, not address: release codegen can merge
/// identical bodies.
#[derive(Clone, Copy)]
enum ListenerIdentity {
    Closure,
    HandleMethod(HandleId, TypeId),
}

impl Listener {
    /// Wraps a closure as a listener.
    pub fn new(callback: impl Fn(&mut App) + 'static) -> Listener {
        Listener {
            callback: Rc::new(callback),
            identity: ListenerIdentity::Closure,
        }
    }

    /// A handle's associated function, comparable like a Dart method tear-off.
    ///
    /// Pass the function by name. Rebuild at the removal site with the same
    /// handle and function. A fn pointer or capturing closure is rejected: it
    /// has no canonical identity. Two captureless closure literals also never
    /// match — each has a unique type, as in Dart where only tear-offs
    /// canonicalize.
    pub fn handle_method<T: 'static, F>(this: Handle<T>, f: F) -> Listener
    where
        F: Fn(Handle<T>, &mut App) + 'static,
    {
        const {
            assert!(
                std::mem::size_of::<F>() == 0,
                "pass the function by name: a fn pointer or capturing closure \
                 has no canonical identity to match on removal"
            )
        };
        Listener {
            callback: Rc::new(move |app: &mut App| f(this, app)),
            identity: ListenerIdentity::HandleMethod(this.id(), TypeId::of::<F>()),
        }
    }

    /// Calls the closure.
    pub fn call(&self, app: &mut App) {
        (self.callback)(app);
    }
}

impl PartialEq for Listener {
    fn eq(&self, other: &Listener) -> bool {
        match (self.identity, other.identity) {
            (ListenerIdentity::Closure, ListenerIdentity::Closure) => {
                Rc::ptr_eq(&self.callback, &other.callback)
            }
            (
                ListenerIdentity::HandleMethod(handle, function),
                ListenerIdentity::HandleMethod(other_handle, other_function),
            ) => handle == other_handle && function == other_function,
            _ => false,
        }
    }
}

impl Eq for Listener {}

impl Hash for Listener {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.identity {
            ListenerIdentity::Closure => (Rc::as_ptr(&self.callback) as *const ()).hash(state),
            ListenerIdentity::HandleMethod(handle, function) => {
                handle.hash(state);
                function.hash(state);
            }
        }
    }
}

impl Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Listener")
    }
}

/// An object that maintains a list of listeners.
///
/// The listeners are typically used to notify clients that the object has been
/// updated.
///
/// There are two variants of this interface:
///
///  * [`ValueListenable`], an interface that augments the [`Listenable`]
///    interface with the concept of a _current value_.
///
///  * `Animation`, an interface that augments the [`ValueListenable`] interface
///    to add the concept of direction (forward or reverse).
///
/// Many classes in the Flutter API use or implement these interfaces. The
/// following subclasses are especially relevant:
///
///  * [`ChangeNotifier`], which can be mixed in to create objects that
///    implement the [`Listenable`] interface.
///
///  * [`ValueNotifier`], which implements the [`ValueListenable`] interface
///    with a mutable value that triggers the notifications when modified.
///
/// The terms "notify clients", "send notifications", "trigger notifications",
/// and "fire notifications" are used interchangeably.
pub trait Listenable {
    /// Register a closure to be called when the object notifies its listeners.
    fn add_listener(&self, app: &mut App, listener: Listener);

    /// Remove a previously registered closure from the list of closures that the
    /// object notifies.
    fn remove_listener(&self, app: &mut App, listener: &Listener);
}

/// A [`Listenable`] that triggers when any of the given [`Listenable`]s
/// themselves trigger.
///
/// Once it is created, items must not be added to or removed from the list.
/// Doing so will lead to memory leaks or exceptions.
///
/// The list may contain `None`s; they are ignored.
///
/// Dart's `Listenable.merge` factory.
pub struct MergingListenable {
    children: Vec<Option<Rc<dyn Listenable>>>,
}

impl MergingListenable {
    /// Dart `Listenable.merge(listenables)`.
    pub fn new(children: Vec<Option<Rc<dyn Listenable>>>) -> MergingListenable {
        MergingListenable { children }
    }
}

impl Listenable for MergingListenable {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        for child in self.children.iter().flatten() {
            child.add_listener(app, listener.clone());
        }
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        for child in self.children.iter().flatten() {
            child.remove_listener(app, listener);
        }
    }
}

impl Debug for MergingListenable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Listenable::merge([{} children])", self.children.len())
    }
}

/// An interface for implementors of [`Listenable`] that expose a [`value`].
///
/// This interface is implemented by [`ValueNotifier<T>`] and `Animation<T>`, and
/// allows other APIs to accept either of those implementations interchangeably.
///
/// [`value`]: ValueListenable::value
pub trait ValueListenable<T>: Listenable {
    /// The current value of the object.
    ///
    /// When the value changes, the callbacks registered with
    /// [`Listenable::add_listener`] will be invoked.
    fn value<'a>(&'a self, app: &'a App) -> &'a T;
}

/// The fields of Dart's `ChangeNotifier` mixin.
#[derive(Debug, Default)]
pub struct ChangeNotifierData {
    count: usize,
    // Dart keeps `_listeners` as a fixed-length list. `count` is the live
    // length; slots from `count` onward, and slots vacated during a dispatch,
    // are `None`.
    listeners: Vec<Option<Listener>>,
    /// Shared with [`Notification`] so a panic still decrements.
    notification_call_stack_depth: Rc<Cell<usize>>,
    reentrantly_removed_listeners: usize,
    #[cfg(debug_assertions)]
    debug_disposed: bool,
}

impl ChangeNotifierData {
    /// Creates a notifier with no listeners.
    pub fn new() -> ChangeNotifierData {
        ChangeNotifierData::default()
    }

    /// Used by owners to assert that the [`ChangeNotifier`] has not yet been
    /// disposed.
    ///
    /// Call it inside a `debug_assert!`, as Dart calls it inside an `assert`,
    /// so that it is stripped in release:
    ///
    /// ```
    /// # use reveal_foundation::ChangeNotifierData;
    /// # struct MyNotifier { change_notifier: ChangeNotifierData }
    /// impl MyNotifier {
    ///     fn do_update(&mut self) {
    ///         debug_assert!(ChangeNotifierData::debug_assert_not_disposed(&self.change_notifier));
    ///         // ...
    ///     }
    /// }
    /// ```
    pub fn debug_assert_not_disposed(notifier: &ChangeNotifierData) -> bool {
        #[cfg(debug_assertions)]
        assert!(
            !notifier.debug_disposed,
            "A ChangeNotifier was used after being disposed.\n\
             Once you have called dispose() on a ChangeNotifier, it can no longer be used."
        );
        let _ = notifier;
        true
    }

    /// Whether any listeners are currently registered.
    ///
    /// Clients should not depend on this value for their behavior, because having
    /// one listener's logic change when another listener happens to start or stop
    /// listening will lead to extremely hard-to-track bugs. Owners might use
    /// this information to determine whether to do any work when there are no
    /// listeners, however; for example, resuming a stream when a listener is
    /// added and pausing it when a listener is removed.
    ///
    /// Typically this is used by overriding [`Listenable::add_listener`], checking
    /// if [`has_listeners`] is false before calling `self.change_notifier.add_listener()`,
    /// and if so, starting whatever work is needed to determine when to call
    /// [`notify_listeners`]; and similarly, by overriding
    /// [`Listenable::remove_listener`], checking if [`has_listeners`] is false
    /// after calling `self.change_notifier.remove_listener()`, and if so, stopping that
    /// same work.
    ///
    /// This method returns false if [`dispose`] has been called.
    ///
    /// [`has_listeners`]: ChangeNotifierData::has_listeners
    /// [`notify_listeners`]: Handle::notify_listeners
    /// [`dispose`]: ChangeNotifierData::dispose
    pub fn has_listeners(&self) -> bool {
        self.count > 0
    }

    /// Register a closure to be called when the object changes.
    ///
    /// If the given closure is already registered, an additional instance is
    /// added, and must be removed the same number of times it is added before it
    /// will stop being called.
    ///
    /// This method must not be called after [`dispose`] has been called.
    ///
    /// If a listener is added twice, and is removed once during an iteration
    /// (e.g. in response to a notification), it will still be called again. If,
    /// on the other hand, it is removed as many times as it was registered, then
    /// it will no longer be called. This odd behavior is the result of the
    /// [`ChangeNotifier`] not being able to determine which listener is being
    /// removed, since they are identical, therefore it will conservatively still
    /// call all the listeners when it knows that any are still registered.
    ///
    /// This surprising behavior can be unexpectedly observed when registering a
    /// listener on two separate objects which are both forwarding all
    /// registrations to a common upstream object.
    ///
    /// See also:
    ///
    ///  * [`remove_listener`], which removes a previously registered closure from
    ///    the list of closures that are notified when the object changes.
    ///
    /// [`dispose`]: ChangeNotifierData::dispose
    /// [`remove_listener`]: ChangeNotifierData::remove_listener
    pub fn add_listener(&mut self, listener: Listener) {
        debug_assert!(ChangeNotifierData::debug_assert_not_disposed(self));

        if self.count == self.listeners.len() {
            if self.count == 0 {
                self.listeners = vec![None; 1];
            } else {
                let mut new_listeners = vec![None; self.listeners.len() * 2];
                new_listeners[..self.count].clone_from_slice(&self.listeners[..self.count]);
                self.listeners = new_listeners;
            }
        }
        self.listeners[self.count] = Some(listener);
        self.count += 1;
    }

    fn remove_at(&mut self, index: usize) {
        // The list holding the listeners is not growable for performances reasons.
        // We still want to shrink this list if a lot of listeners have been added
        // and then removed outside a notify_listeners iteration.
        // We do this only when the real number of listeners is half the length
        // of our list.
        self.count -= 1;
        if self.count * 2 <= self.listeners.len() {
            let mut new_listeners = vec![None; self.count];

            // Listeners before the index are at the same place.
            new_listeners[..index].clone_from_slice(&self.listeners[..index]);

            // Listeners after the index move towards the start of the list.
            new_listeners[index..self.count]
                .clone_from_slice(&self.listeners[index + 1..self.count + 1]);

            self.listeners = new_listeners;
        } else {
            // When there are more listeners than half the length of the list, we only
            // shift our listeners, so that we avoid to reallocate memory for the
            // whole list.
            for i in index..self.count {
                let moved = self.listeners[i + 1].clone();
                self.listeners[i] = moved;
            }
            self.listeners[self.count] = None;
        }
    }

    /// Remove a previously registered closure from the list of closures that are
    /// notified when the object changes.
    ///
    /// If the given listener is not registered, the call is ignored.
    ///
    /// This method returns immediately if [`dispose`] has been called.
    ///
    /// If a listener is added twice, and is removed once during an iteration
    /// (e.g. in response to a notification), it will still be called again. If,
    /// on the other hand, it is removed as many times as it was registered, then
    /// it will no longer be called. This odd behavior is the result of the
    /// [`ChangeNotifier`] not being able to determine which listener is being
    /// removed, since they are identical, therefore it will conservatively still
    /// call all the listeners when it knows that any are still registered.
    ///
    /// See also:
    ///
    ///  * [`add_listener`], which registers a closure to be called when the object
    ///    changes.
    ///
    /// [`dispose`]: ChangeNotifierData::dispose
    /// [`add_listener`]: ChangeNotifierData::add_listener
    pub fn remove_listener(&mut self, listener: &Listener) {
        // This method is allowed to be called on disposed instances for usability
        // reasons. Due to how our frame scheduling logic between render objects and
        // overlays, it is common that the owner of this instance would be disposed a
        // frame earlier than the listeners. Allowing calls to this method after it
        // is disposed makes it easier for listeners to properly clean up.
        for i in 0..self.count {
            if self.listeners[i].as_ref() == Some(listener) {
                if self.notification_call_stack_depth.get() > 0 {
                    // We don't resize the list during notify_listeners iterations
                    // but we set to null, the listeners we want to remove. We will
                    // effectively resize the list at the end of all notify_listeners
                    // iterations.
                    self.listeners[i] = None;
                    self.reentrantly_removed_listeners += 1;
                } else {
                    // When we are outside the notify_listeners iterations we can
                    // effectively shrink the list.
                    self.remove_at(i);
                }
                break;
            }
        }
    }

    /// Discards any resources used by the object.
    ///
    /// After this is called, the object is not in a usable state and should be
    /// discarded (calls to [`add_listener`] will panic in debug builds after the
    /// object is disposed).
    ///
    /// This method should only be called by the object's owner.
    ///
    /// This method does not notify listeners, and clears the listener list once
    /// it is called. Owners of this type must decide on whether to notify
    /// listeners or not immediately before disposal.
    ///
    /// [`add_listener`]: ChangeNotifierData::add_listener
    pub fn dispose(&mut self) {
        debug_assert!(ChangeNotifierData::debug_assert_not_disposed(self));
        debug_assert!(
            self.notification_call_stack_depth.get() == 0,
            "The \"dispose()\" method on {self:?} was called during the call to \
             \"notify_listeners()\". This is likely to cause errors since it modifies \
             the list of listeners while the list is being used."
        );
        #[cfg(debug_assertions)]
        {
            self.debug_disposed = true;
        }
        self.listeners = Vec::new();
        self.count = 0;
        // A panicking listener skips Dart's per-listener `catch`, which is
        // what zeros this. Leaving it set underflows
        // `count - reentrantly_removed_listeners` on the next dispatch.
        //
        // The depth is not reset: it is shared with any live Notification,
        // which decrements on drop.
        self.reentrantly_removed_listeners = 0;
    }

    /// Dispatch through [`Handle::notify_listeners`]. The owner is a handle, not
    /// `&mut self`, so a listener can re-enter it.
    pub(crate) fn notify_listeners<T: 'static>(
        app: &mut App,
        owner: Handle<T>,
        field: fn(&mut T) -> &mut ChangeNotifierData,
    ) {
        let (notification, end) = {
            let notifier = field(app.get_mut(owner));
            debug_assert!(ChangeNotifierData::debug_assert_not_disposed(notifier));
            if notifier.count == 0 {
                return;
            }

            // To make sure that listeners removed during this iteration are not called,
            // we set them to null, but we don't shrink the list right away.
            // By doing this, we can continue to iterate on our list until it reaches
            // the last listener added before the call to this method.

            // To allow potential listeners to recursively call notify_listeners, we track
            // the number of times this method is called in notification_call_stack_depth.
            // Once every recursive iteration is finished (i.e. when
            // notification_call_stack_depth == 0), we can safely shrink our list so that
            // it will only contain not null listeners.
            let notification =
                Notification::begin(Rc::clone(&notifier.notification_call_stack_depth));
            (notification, notifier.count)
        };

        for i in 0..end {
            let listener = field(app.get_mut(owner)).listeners[i].clone();
            if let Some(listener) = listener {
                listener.call(app);
            }
        }

        drop(notification);

        let notifier = field(app.get_mut(owner));
        if notifier.notification_call_stack_depth.get() == 0
            && notifier.reentrantly_removed_listeners > 0
        {
            notifier.compact_reentrantly_removed_listeners();
        }
    }

    fn compact_reentrantly_removed_listeners(&mut self) {
        // We really remove the listeners when all notifications are done.
        let new_length = self.count - self.reentrantly_removed_listeners;
        if new_length * 2 <= self.listeners.len() {
            // As in remove_at, we only shrink the list when the real number of
            // listeners is half the length of our list.
            let mut new_listeners = vec![None; new_length];

            let mut new_index = 0;
            for listener in &self.listeners[..self.count] {
                if listener.is_some() {
                    new_listeners[new_index] = listener.clone();
                    new_index += 1;
                }
            }

            self.listeners = new_listeners;
        } else {
            // Otherwise we put all the null references at the end.
            for i in 0..new_length {
                if self.listeners[i].is_none() {
                    // We swap this item with the next not null item.
                    let mut swap_index = i + 1;
                    while self.listeners[swap_index].is_none() {
                        swap_index += 1;
                    }
                    self.listeners.swap(i, swap_index);
                }
            }
        }

        self.reentrantly_removed_listeners = 0;
        self.count = new_length;
    }
}

/// Dart's `try`/`finally` around each listener. Holds the depth by `Rc`, never
/// a borrow of the notifier, so a panic still decrements.
struct Notification {
    depth: Rc<Cell<usize>>,
}

impl Notification {
    fn begin(depth: Rc<Cell<usize>>) -> Notification {
        depth.set(depth.get() + 1);
        Notification { depth }
    }
}

impl Drop for Notification {
    fn drop(&mut self) {
        self.depth.set(self.depth.get() - 1);
    }
}

/// The object side of [`Listenable`]: an arena object that keeps listeners. Implementing it
/// makes `Handle<Self>` a [`Listenable`]. Every [`ChangeNotifier`] is one; an object with its
/// own listener bookkeeping (Flutter's `AnimationLocalListenersMixin`) implements it directly.
///
/// [`Listenable`] itself takes `&self` so that an erased handle type can implement it; a crate
/// cannot implement that foreign trait for `Handle<ItsType>`, so it implements this one on
/// the type instead.
pub trait ListenableObject: Sized + 'static {
    /// Register a closure to be called when the object notifies its listeners.
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener);

    /// Remove a previously registered closure from the list of closures that the
    /// object notifies.
    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener);
}

impl<T: ChangeNotifier> ListenableObject for T {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        ChangeNotifier::will_add_listener(self, app);
        app.get_mut(self)
            .change_notifier_data_mut()
            .add_listener(listener);
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        // Allowed on a disposed instance (see `ChangeNotifierData::remove_listener`); here a
        // disposed notifier may already have left the arena, so a stale handle is a no-op.
        if !app.contains(self) {
            return;
        }
        app.get_mut(self)
            .change_notifier_data_mut()
            .remove_listener(listener);
        ChangeNotifier::did_remove_listener(self, app);
    }
}

impl<T: ListenableObject> Listenable for Handle<T> {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        T::add_listener(*self, app, listener);
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        T::remove_listener(*self, app, listener);
    }
}

/// A class that can be mixed in that provides a change notification API using
/// [`Listener`] for notifications.
///
/// It is O(1) for adding listeners and O(N) for removing listeners and
/// dispatching notifications (where N is the number of listeners).
///
/// Hold [`ChangeNotifierData`] as a field and implement this trait. Then
/// [`Handle::notify_listeners`] is inherent — no extra import at the call site.
///
/// See also:
///
///  * [`ValueNotifier`], which is a [`ChangeNotifier`] that wraps a single
///    value.
pub trait ChangeNotifier: Sized + 'static {
    fn change_notifier_data(&self) -> &ChangeNotifierData;

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData;

    /// Runs before the listener is stored. Dart overrides of `addListener` that
    /// work before `super.addListener` go here.
    fn will_add_listener(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// Runs after the listener is removed. Dart overrides of `removeListener` that
    /// work after `super.removeListener` go here.
    fn did_remove_listener(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }
}

impl ChangeNotifier for ChangeNotifierData {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        self
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        self
    }
}

impl<T: ChangeNotifier> Handle<T> {
    /// Call all the registered listeners.
    ///
    /// Call this method whenever the object changes, to notify any clients the
    /// object may have changed. Listeners that are added during this iteration
    /// will not be visited. Listeners that are removed during this iteration will
    /// not be visited after they are removed.
    ///
    /// This method must not be called after [`ChangeNotifierData::dispose`] has been
    /// called.
    ///
    /// Surprising behavior can result when reentrantly removing a listener (e.g.
    /// in response to a notification) that has been registered multiple times.
    /// See the discussion at [`ChangeNotifierData::remove_listener`].
    pub fn notify_listeners(self, app: &mut App) {
        ChangeNotifierData::notify_listeners(app, self, T::change_notifier_data_mut);
    }
}

/// A [`ChangeNotifier`] that holds a single value.
///
/// When [`value`] is replaced with a new value that is **not equal** to the old
/// value as evaluated by `PartialEq`, this type notifies its listeners.
///
/// ## Limitations
///
/// Notifications are triggered based on **equality**, not on mutations within
/// the value itself. As a result, changes to mutable values that do not affect
/// their equality will not cause listeners to be notified.
///
/// Because of this behavior, [`ValueNotifier`] is best used with immutable data
/// types. For mutable data types, consider holding a [`ChangeNotifierData`]
/// directly and calling [`Handle::notify_listeners`] when changes occur.
///
/// [`value`]: ValueListenable::value
#[derive(Debug, Default)]
pub struct ValueNotifier<T> {
    change_notifier: ChangeNotifierData,
    value: T,
}

impl<T> ValueNotifier<T> {
    /// Creates a [`ChangeNotifier`] that wraps this value.
    pub fn new(value: T) -> ValueNotifier<T> {
        ValueNotifier {
            change_notifier: ChangeNotifierData::new(),
            value,
        }
    }

    /// The wrapped value. Prefer [`ValueListenable::value`] on the handle when
    /// you do not already have a borrow of the slot.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Discards any resources used by the object.
    pub fn dispose(&mut self) {
        self.change_notifier.dispose();
    }
}

impl<T: 'static> ChangeNotifier for ValueNotifier<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl<T: PartialEq + 'static> Handle<ValueNotifier<T>> {
    /// Replaces the current value, notifying listeners when it differs.
    ///
    /// Dart's setter takes only the value. This takes [`App`] because notifying
    /// does: a listener may assign back through this setter.
    pub fn set_value(self, app: &mut App, new_value: T) {
        if app.get(self).value == new_value {
            return;
        }
        app.get_mut(self).value = new_value;
        self.notify_listeners(app);
    }
}

impl<T: 'static> ValueListenable<T> for Handle<ValueNotifier<T>> {
    fn value<'a>(&'a self, app: &'a App) -> &'a T {
        app.get(*self).value()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::app_cell::AppCell;

    type Log = Rc<RefCell<Vec<String>>>;

    fn fired(log: &Log) -> Vec<String> {
        log.borrow().clone()
    }

    #[derive(Default)]
    struct TestNotifier {
        change_notifier: ChangeNotifierData,
    }

    impl ChangeNotifier for TestNotifier {
        fn change_notifier_data(&self) -> &ChangeNotifierData {
            &self.change_notifier
        }

        fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
            &mut self.change_notifier
        }
    }

    fn recording(log: &Log, name: &'static str) -> Listener {
        let log = Rc::clone(log);
        Listener::new(move |_app| log.borrow_mut().push(name.to_string()))
    }

    fn add(app: &mut App, host: Handle<TestNotifier>, listener: Listener) {
        host.add_listener(app, listener);
    }

    fn remove(app: &mut App, host: Handle<TestNotifier>, listener: &Listener) {
        host.remove_listener(app, listener);
    }

    fn notify(app: &mut App, host: Handle<TestNotifier>) {
        host.notify_listeners(app);
    }

    fn silence_panic_hook() -> Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static> {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        hook
    }

    /// `change_notifier_test.dart`: "ChangeNotifier" — order, duplicates, extra remove.
    #[test]
    fn listeners_are_called_in_the_order_they_were_added() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let listener = recording(&log, "listener");
        add(&mut app, host, listener.clone());
        add(&mut app, host, listener.clone());
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener", "listener"]);
        log.borrow_mut().clear();

        remove(&mut app, host, &listener);
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener"]);
        log.borrow_mut().clear();

        remove(&mut app, host, &listener);
        notify(&mut app, host);
        assert_eq!(fired(&log), Vec::<String>::new());
        log.borrow_mut().clear();

        remove(&mut app, host, &listener);
        notify(&mut app, host);
        assert_eq!(fired(&log), Vec::<String>::new());
        log.borrow_mut().clear();

        add(&mut app, host, listener.clone());
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener"]);
        log.borrow_mut().clear();

        let listener1 = recording(&log, "listener1");
        add(&mut app, host, listener1.clone());
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener", "listener1"]);
        log.borrow_mut().clear();

        let listener2 = recording(&log, "listener2");
        add(&mut app, host, listener2.clone());
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener", "listener1", "listener2"]);
        log.borrow_mut().clear();

        remove(&mut app, host, &listener1);
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener", "listener2"]);
        log.borrow_mut().clear();

        add(&mut app, host, listener1.clone());
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener", "listener2", "listener1"]);
    }

    /// `change_notifier_test.dart`: "ChangeNotifier with mutating listener".
    #[test]
    fn a_listener_mutates_the_notifier_dispatching_to_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let listener1 = recording(&log, "listener1");
        let listener3 = recording(&log, "listener3");
        let listener4 = recording(&log, "listener4");
        let listener2 = {
            let log = Rc::clone(&log);
            let (one, three, four) = (listener1.clone(), listener3.clone(), listener4.clone());
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("listener2".to_string());
                let notifier = &mut app.get_mut(host).change_notifier;
                notifier.remove_listener(&one);
                notifier.remove_listener(&three);
                notifier.add_listener(four.clone());
            })
        };

        add(&mut app, host, listener1);
        add(&mut app, host, listener2);
        add(&mut app, host, listener3);

        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener1", "listener2"]);

        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener2", "listener4"]);

        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener2", "listener4", "listener4"]);
    }

    /// `change_notifier_test.dart`: "During notifyListeners, a listener was added and removed immediately".
    #[test]
    fn a_listener_added_and_removed_immediately_during_notify_is_not_called() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let listener2 = recording(&log, "listener2");
        let listener3 = recording(&log, "listener3");
        let listener1 = {
            let log = Rc::clone(&log);
            let two = listener2.clone();
            let three = listener3.clone();
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("listener1".to_string());
                let notifier = &mut app.get_mut(host).change_notifier;
                notifier.add_listener(two.clone());
                notifier.remove_listener(&two);
                notifier.add_listener(three.clone());
            })
        };

        add(&mut app, host, listener1);
        notify(&mut app, host);
        assert_eq!(fired(&log), ["listener1"]);
    }

    /// `change_notifier_test.dart`: self-removing listener in the middle still notifies all.
    #[test]
    fn a_self_removing_listener_in_the_middle_still_notifies_all() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let listener1 = recording(&log, "listener1");
        let self_removing_slot: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
        let self_removing = {
            let log = Rc::clone(&log);
            let slot = Rc::clone(&self_removing_slot);
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("selfRemovingListener".to_string());
                let me = slot.borrow().clone().unwrap();
                app.get_mut(host).change_notifier.remove_listener(&me);
            })
        };
        *self_removing_slot.borrow_mut() = Some(self_removing.clone());

        add(&mut app, host, listener1.clone());
        add(&mut app, host, self_removing);
        add(&mut app, host, listener1);

        notify(&mut app, host);
        assert_eq!(
            fired(&log),
            ["listener1", "selfRemovingListener", "listener1"]
        );
    }

    /// `change_notifier_test.dart`: first listener removes itself, still notifies the rest.
    #[test]
    fn the_first_listener_removing_itself_still_notifies_the_rest() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let self_removing_slot: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
        let self_removing = {
            let log = Rc::clone(&log);
            let slot = Rc::clone(&self_removing_slot);
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("selfRemovingListener".to_string());
                let me = slot.borrow().clone().unwrap();
                app.get_mut(host).change_notifier.remove_listener(&me);
            })
        };
        *self_removing_slot.borrow_mut() = Some(self_removing.clone());
        let listener1 = recording(&log, "listener1");

        add(&mut app, host, self_removing);
        add(&mut app, host, listener1);

        notify(&mut app, host);
        assert_eq!(fired(&log), ["selfRemovingListener", "listener1"]);
    }

    #[test]
    fn only_a_handle_to_the_same_closure_compares_equal() {
        let log = Log::default();
        let listener = recording(&log, "a");

        assert_eq!(listener, listener.clone());
        assert_ne!(listener, recording(&log, "a"));
    }

    #[test]
    fn a_rebuilt_handle_method_listener_matches_like_a_dart_tear_off() {
        struct Host;
        fn hook(_this: Handle<Host>, _app: &mut App) {}
        fn other_hook(_this: Handle<Host>, _app: &mut App) {}

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let a = app.create(Host);
        let b = app.create(Host);

        assert_eq!(
            Listener::handle_method(a, hook),
            Listener::handle_method(a, hook)
        );
        assert_ne!(
            Listener::handle_method(a, hook),
            Listener::handle_method(b, hook)
        );
        assert_ne!(
            Listener::handle_method(a, hook),
            Listener::handle_method(a, other_hook)
        );
        assert_ne!(Listener::handle_method(a, hook), Listener::new(|_app| {}));
    }

    #[test]
    fn a_handle_method_listener_calls_the_function_with_its_handle() {
        struct Host;
        thread_local! {
            static CALLED_WITH: Cell<Option<HandleId>> = const { Cell::new(None) };
        }
        fn hook(this: Handle<Host>, _app: &mut App) {
            CALLED_WITH.with(|called| called.set(Some(this.id())));
        }

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let host = app.create(Host);
        Listener::handle_method(host, hook).call(&mut app);
        assert_eq!(CALLED_WITH.with(Cell::get), Some(host.id()));
    }

    /// `remove_at` shifts in place while survivors fill more than half the
    /// buffer, and reallocates once they fit in half of it.
    #[test]
    fn removing_keeps_order_across_both_shrink_branches() {
        for first in 0..8 {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let log = Log::default();
            let host = app.create(TestNotifier::default());

            let names = ["0", "1", "2", "3", "4", "5", "6", "7"];
            let listeners: Vec<Listener> = names.iter().map(|name| recording(&log, name)).collect();
            for listener in &listeners {
                add(&mut app, host, listener.clone());
            }

            let mut survivors: Vec<String> = names.iter().map(|n| n.to_string()).collect();
            let mut saw_realloc = false;

            for step in 0..8 {
                let index = (first + step) % 8;
                remove(&mut app, host, &listeners[index]);
                survivors.retain(|s| s != names[index]);
                saw_realloc |= app.get(host).change_notifier.listeners.len() < 8;

                log.borrow_mut().clear();
                notify(&mut app, host);
                assert_eq!(fired(&log), survivors, "after removing {index}");
                assert_eq!(app.get(host).change_notifier.count, survivors.len());
            }

            assert!(saw_realloc, "the shrink branch never ran");
        }
    }

    /// `change_notifier_test.dart`: "Cannot use a disposed ChangeNotifier except for remove listener".
    #[test]
    fn dispose_drops_every_listener_but_leaves_removal_callable() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let listener = recording(&log, "a");
        add(&mut app, host, listener.clone());
        app.get_mut(host).change_notifier.dispose();

        assert!(!app.get(host).change_notifier.has_listeners());
        app.get_mut(host).change_notifier.remove_listener(&listener);
        assert_eq!(fired(&log), Vec::<String>::new());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "was used after being disposed")]
    fn adding_a_listener_after_dispose_panics_in_debug() {
        let mut notifier = ChangeNotifierData::new();
        notifier.dispose();
        notifier.add_listener(recording(&Log::default(), "a"));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "was used after being disposed")]
    fn notifying_after_dispose_panics_in_debug() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let host = app.create(TestNotifier::default());
        app.get_mut(host).change_notifier.dispose();
        notify(&mut app, host);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "was used after being disposed")]
    fn disposing_twice_panics_in_debug() {
        let mut notifier = ChangeNotifierData::new();
        notifier.dispose();
        notifier.dispose();
    }

    /// `change_notifier_test.dart`: "Can check hasListener on a disposed ChangeNotifier".
    #[test]
    fn has_listeners_is_false_after_dispose() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.create(ValueNotifier::new(0i32));
        counter.add_listener(&mut app, recording(&Log::default(), "a"));
        assert!(app.get(counter).change_notifier.has_listeners());
        app.get_mut(counter).dispose();
        assert!(!app.get(counter).change_notifier.has_listeners());
    }

    /// `change_notifier_test.dart`: "Value notifier".
    #[test]
    fn value_notifier_notifies_only_when_the_value_differs() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let notifier = app.create(ValueNotifier::new(2.0));

        notifier.add_listener(&mut app, {
            let log = Rc::clone(&log);
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push(app.get(notifier).value().to_string());
            })
        });

        notifier.set_value(&mut app, 3.0);
        assert_eq!(fired(&log), ["3"]);
        log.borrow_mut().clear();

        notifier.set_value(&mut app, 3.0);
        assert_eq!(fired(&log), Vec::<String>::new());
    }

    /// `change_notifier_test.dart`: "hasListeners".
    #[test]
    fn has_listeners() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let notifier = app.create(ValueNotifier::new(true));
        assert!(!app.get(notifier).change_notifier.has_listeners());

        let test1 = recording(&Log::default(), "test1");
        let test2 = recording(&Log::default(), "test2");

        notifier.add_listener(&mut app, test1.clone());
        assert!(app.get(notifier).change_notifier.has_listeners());
        notifier.add_listener(&mut app, test1.clone());
        assert!(app.get(notifier).change_notifier.has_listeners());
        notifier.remove_listener(&mut app, &test1);
        assert!(app.get(notifier).change_notifier.has_listeners());
        notifier.remove_listener(&mut app, &test1);
        assert!(!app.get(notifier).change_notifier.has_listeners());
        notifier.add_listener(&mut app, test1.clone());
        assert!(app.get(notifier).change_notifier.has_listeners());
        notifier.add_listener(&mut app, test2.clone());
        assert!(app.get(notifier).change_notifier.has_listeners());
        notifier.remove_listener(&mut app, &test1);
        assert!(app.get(notifier).change_notifier.has_listeners());
        notifier.remove_listener(&mut app, &test2);
        assert!(!app.get(notifier).change_notifier.has_listeners());
    }

    /// `change_notifier_test.dart`: "Calling debugAssertNotDisposed works as intended".
    #[test]
    fn debug_assert_not_disposed_returns_true_while_live() {
        let notifier = ChangeNotifierData::new();
        assert!(ChangeNotifierData::debug_assert_not_disposed(&notifier));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "was used after being disposed")]
    fn debug_assert_not_disposed_panics_after_dispose() {
        let mut notifier = ChangeNotifierData::new();
        notifier.dispose();
        ChangeNotifierData::debug_assert_not_disposed(&notifier);
    }

    /// `change_notifier_test.dart`: "notifyListener can be called recursively".
    #[test]
    fn a_listener_assigns_through_its_owners_setter_and_the_dispatch_recurses() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let counter = app.create(ValueNotifier::new(0i32));

        let listener1 = {
            let log = Rc::clone(&log);
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("listener1".to_string());
                if *app.get(counter).value() < 0 {
                    counter.set_value(app, 0);
                }
            })
        };
        counter.add_listener(&mut app, listener1);

        counter.notify_listeners(&mut app);
        assert_eq!(fired(&log), ["listener1"]);

        log.borrow_mut().clear();
        counter.set_value(&mut app, 3);
        assert_eq!(fired(&log), ["listener1"]);

        log.borrow_mut().clear();
        counter.set_value(&mut app, -2);
        assert_eq!(fired(&log), ["listener1", "listener1"]);
        assert_eq!(*app.get(counter).value(), 0);
    }

    /// `change_notifier_test.dart`: "Remove Listeners while notifying on a list which will not resize".
    #[test]
    fn removing_listeners_while_notifying_on_a_list_which_will_not_resize() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let names = [
            "listener0",
            "listener1",
            "listener2",
            "listener3",
            "listener4",
            "listener5",
            "listener6",
            "listener7",
            "listener8",
            "listener9",
            "listener10",
            "listener11",
        ];
        let listeners: Vec<Listener> = names.iter().map(|name| recording(&log, name)).collect();

        let auto_remove = {
            let to_remove = [
                listeners[1].clone(),
                listeners[3].clone(),
                listeners[4].clone(),
            ];
            let auto_remove_slot: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
            let slot = Rc::clone(&auto_remove_slot);
            let listener = Listener::new(move |app: &mut App| {
                let notifier = &mut app.get_mut(host).change_notifier;
                notifier.remove_listener(&to_remove[0]);
                notifier.remove_listener(&to_remove[1]);
                notifier.remove_listener(&to_remove[2]);
                let me = slot.borrow().clone().unwrap();
                notifier.remove_listener(&me);
            });
            *auto_remove_slot.borrow_mut() = Some(listener.clone());
            listener
        };

        add(&mut app, host, auto_remove);
        for listener in &listeners {
            add(&mut app, host, listener.clone());
        }

        let remaining = [
            "listener0",
            "listener2",
            "listener5",
            "listener6",
            "listener7",
            "listener8",
            "listener9",
            "listener10",
            "listener11",
        ];

        notify(&mut app, host);
        assert_eq!(fired(&log), remaining);

        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(fired(&log), remaining);

        for i in [0, 2, 5, 6, 7, 8, 9, 10, 11] {
            remove(&mut app, host, &listeners[i]);
        }
        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(fired(&log), Vec::<String>::new());
    }

    /// `change_notifier_test.dart`: "ChangeNotifier can not dispose in callback".
    #[test]
    #[cfg(debug_assertions)]
    fn dispose_during_notify_panics_and_does_not_finish_the_callback() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let host = app.create(TestNotifier::default());
        let finished = Rc::new(Cell::new(false));
        let finished_flag = Rc::clone(&finished);
        add(
            &mut app,
            host,
            Listener::new(move |app: &mut App| {
                app.get_mut(host).change_notifier.dispose();
                finished_flag.set(true);
            }),
        );

        let hook = silence_panic_hook();
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            notify(&mut app, host);
        }))
        .is_err();
        std::panic::set_hook(hook);

        assert!(panicked);
        assert!(!finished.get());
        app.get_mut(host).change_notifier.dispose();
    }

    #[test]
    fn a_bare_change_notifier_handle_can_notify() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let on_undo = app.create(ChangeNotifierData::new());

        on_undo.add_listener(&mut app, recording(&log, "undo"));
        on_undo.notify_listeners(&mut app);

        assert_eq!(fired(&log), ["undo"]);
    }

    #[test]
    fn a_panicking_listener_leaves_the_notifier_usable() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let l0 = recording(&log, "l0");
        let l2 = recording(&log, "l2");
        let l3 = Listener::new(|_app: &mut App| panic!("bad listener"));
        let l1 = {
            let log = Rc::clone(&log);
            let (zero, two) = (l0.clone(), l2.clone());
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("l1".to_string());
                let notifier = &mut app.get_mut(host).change_notifier;
                notifier.remove_listener(&zero);
                notifier.remove_listener(&two);
            })
        };

        add(&mut app, host, l0);
        add(&mut app, host, l1);
        add(&mut app, host, l2);
        add(&mut app, host, l3.clone());

        let hook = silence_panic_hook();
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            notify(&mut app, host);
        }))
        .is_err();
        std::panic::set_hook(hook);

        assert!(panicked);
        assert_eq!(fired(&log), ["l0", "l1"]);
        assert_eq!(
            app.get(host)
                .change_notifier
                .notification_call_stack_depth
                .get(),
            0
        );
        assert_eq!(app.get(host).change_notifier.count, 4);

        app.get_mut(host).change_notifier.remove_listener(&l3);
        assert_eq!(app.get(host).change_notifier.count, 3);
        assert_eq!(
            app.get(host).change_notifier.reentrantly_removed_listeners,
            2
        );

        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(fired(&log), ["l1"]);
        assert_eq!(app.get(host).change_notifier.count, 1);
        assert_eq!(
            app.get(host).change_notifier.reentrantly_removed_listeners,
            0
        );

        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(fired(&log), ["l1"]);
    }

    #[test]
    fn a_panic_inside_a_nested_dispatch_still_pairs_the_depth_guards() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());
        let inner_host = app.create(TestNotifier::default());

        let doomed = recording(&log, "doomed");
        add(
            &mut app,
            inner_host,
            Listener::new(|_app: &mut App| panic!("inner listener")),
        );

        let outer = {
            let log = Rc::clone(&log);
            let doomed = doomed.clone();
            Listener::new(move |app: &mut App| {
                log.borrow_mut().push("outer".to_string());
                app.get_mut(host).change_notifier.remove_listener(&doomed);
                notify(app, inner_host);
            })
        };

        add(&mut app, host, outer);
        add(&mut app, host, doomed);

        let hook = silence_panic_hook();
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            notify(&mut app, host);
        }))
        .is_err();
        std::panic::set_hook(hook);

        assert!(panicked);
        assert_eq!(
            app.get(host)
                .change_notifier
                .notification_call_stack_depth
                .get(),
            0
        );
        assert_eq!(
            app.get(inner_host)
                .change_notifier
                .notification_call_stack_depth
                .get(),
            0
        );

        let survivor = app.get(host).change_notifier.listeners[0].clone().unwrap();
        remove(&mut app, host, &survivor);
        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(
            app.get(host).change_notifier.reentrantly_removed_listeners,
            0
        );
        assert!(!app.get(host).change_notifier.has_listeners());
    }

    #[test]
    fn dispose_clears_the_compaction_owed_by_a_panicked_dispatch() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let doomed = recording(&log, "doomed");
        let saboteur = {
            let doomed = doomed.clone();
            Listener::new(move |app: &mut App| {
                app.get_mut(host).change_notifier.remove_listener(&doomed);
                panic!("bad listener");
            })
        };
        add(&mut app, host, saboteur);
        add(&mut app, host, doomed);

        let hook = silence_panic_hook();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            notify(&mut app, host);
        }));
        std::panic::set_hook(hook);

        assert_eq!(
            app.get(host).change_notifier.reentrantly_removed_listeners,
            1
        );

        app.get_mut(host).change_notifier.dispose();
        assert_eq!(
            app.get(host).change_notifier.reentrantly_removed_listeners,
            0
        );
        assert_eq!(app.get(host).change_notifier.count, 0);
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn a_disposed_notifier_reused_in_release_does_not_underflow() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let host = app.create(TestNotifier::default());

        let doomed = recording(&log, "doomed");
        let saboteur = {
            let doomed = doomed.clone();
            Listener::new(move |app: &mut App| {
                app.get_mut(host).change_notifier.remove_listener(&doomed);
                panic!("bad listener");
            })
        };
        add(&mut app, host, saboteur);
        add(&mut app, host, doomed);

        let hook = silence_panic_hook();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            notify(&mut app, host);
        }));
        std::panic::set_hook(hook);

        app.get_mut(host).change_notifier.dispose();
        add(&mut app, host, recording(&log, "after"));

        log.borrow_mut().clear();
        notify(&mut app, host);
        assert_eq!(fired(&log), ["after"]);
        assert_eq!(app.get(host).change_notifier.count, 1);
    }
}
