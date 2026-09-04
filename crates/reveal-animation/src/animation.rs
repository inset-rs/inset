//! Flutter counterpart: `animation/animation.dart`.

use std::any::{TypeId, type_name};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use reveal_foundation::{App, Handle, HandleId, Listener};

use crate::tween::Animatable;

/// The status of an animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationStatus {
    /// The animation is stopped at the beginning.
    Dismissed,

    /// The animation is running from beginning to end.
    Forward,

    /// The animation is running backwards, from end to beginning.
    Reverse,

    /// The animation is stopped at the end.
    Completed,
}

impl AnimationStatus {
    /// Whether the animation is stopped at the beginning.
    pub fn is_dismissed(self) -> bool {
        self == AnimationStatus::Dismissed
    }

    /// Whether the animation is stopped at the end.
    pub fn is_completed(self) -> bool {
        self == AnimationStatus::Completed
    }

    /// Whether the animation is running in either direction.
    pub fn is_animating(self) -> bool {
        match self {
            AnimationStatus::Forward | AnimationStatus::Reverse => true,
            AnimationStatus::Completed | AnimationStatus::Dismissed => false,
        }
    }

    /// Whether the current aim of the animation is toward completion.
    pub fn is_forward_or_completed(self) -> bool {
        match self {
            AnimationStatus::Forward | AnimationStatus::Completed => true,
            AnimationStatus::Reverse | AnimationStatus::Dismissed => false,
        }
    }
}

/// Signature for listeners attached using [`Animation::add_status_listener`].
///
/// Like [`Listener`], this is a handle matched by identity: a listener over a
/// closure ([`AnimationStatusListener::new`]) is equal only to its own clones,
/// and a listener over an entity's associated function
/// ([`AnimationStatusListener::handle_method`]) is equal to any other built
/// from the same entity and function.
#[derive(Clone)]
pub struct AnimationStatusListener {
    callback: Rc<StatusCallback>,
    identity: StatusListenerIdentity,
}

type StatusCallback = dyn Fn(AnimationStatus, &mut App);

/// See `ListenerIdentity` on [`Listener`] — the same split for the same
/// reason, including the function-`TypeId` half.
#[derive(Clone, Copy)]
enum StatusListenerIdentity {
    Closure,
    HandleMethod(HandleId, TypeId),
}

impl AnimationStatusListener {
    /// Wraps a closure as a status listener.
    pub fn new(callback: impl Fn(AnimationStatus, &mut App) + 'static) -> AnimationStatusListener {
        AnimationStatusListener {
            callback: Rc::new(callback),
            identity: StatusListenerIdentity::Closure,
        }
    }

    /// Wraps an entity's associated function as a status listener.
    ///
    /// The counterpart of Dart passing the `notifyStatusListeners` tear-off;
    /// see [`Listener::handle_method`] for the identity rule. Pass the
    /// function by name — the compile-time assert rejects fn pointers and
    /// capturing closures, which have no canonical identity.
    ///
    /// The function is receiver-first, `(Handle<T>, &mut App, AnimationStatus)`,
    /// so a method on `Handle<T>` can be named directly.
    pub fn handle_method<T: 'static, F>(this: Handle<T>, f: F) -> AnimationStatusListener
    where
        F: Fn(Handle<T>, &mut App, AnimationStatus) + 'static,
    {
        const {
            assert!(
                std::mem::size_of::<F>() == 0,
                "pass the function by name: a fn pointer or capturing closure \
                 has no canonical identity to match on removal"
            )
        };
        AnimationStatusListener {
            callback: Rc::new(move |status, app: &mut App| f(this, app, status)),
            identity: StatusListenerIdentity::HandleMethod(this.id(), TypeId::of::<F>()),
        }
    }

    /// Calls the closure.
    pub fn call(&self, status: AnimationStatus, app: &mut App) {
        (self.callback)(status, app);
    }
}

impl PartialEq for AnimationStatusListener {
    fn eq(&self, other: &AnimationStatusListener) -> bool {
        match (self.identity, other.identity) {
            (StatusListenerIdentity::Closure, StatusListenerIdentity::Closure) => {
                Rc::ptr_eq(&self.callback, &other.callback)
            }
            (
                StatusListenerIdentity::HandleMethod(entity, function),
                StatusListenerIdentity::HandleMethod(other_entity, other_function),
            ) => entity == other_entity && function == other_function,
            _ => false,
        }
    }
}

impl Eq for AnimationStatusListener {}

impl Hash for AnimationStatusListener {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.identity {
            StatusListenerIdentity::Closure => {
                (Rc::as_ptr(&self.callback) as *const ()).hash(state);
            }
            StatusListenerIdentity::HandleMethod(entity, function) => {
                entity.hash(state);
                function.hash(state);
            }
        }
    }
}

impl Debug for AnimationStatusListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AnimationStatusListener")
    }
}

/// A value which might change over time, moving forward or backward.
///
/// This is what a concrete animation implements — Dart's
/// `class X extends Animation<T>`. A field or parameter Dart types as
/// `Animation<T>` holds the erased [`AnyAnimation<T>`] instead; get one from
/// [`as_animation`](Animation::as_animation).
///
/// The members take `self: Handle<Self>` rather than `&self`, for the reason
/// recorded on the `listener_helpers` traits: a `&self` receiver would borrow
/// this animation's slot for the whole call, and the listener members hand
/// `&mut App` onward to reach other objects and run listeners.
pub trait Animation<T: 'static>: Sized + 'static {
    /// Calls the listener every time the value of the animation changes.
    ///
    /// Listeners can be removed with [`remove_listener`].
    ///
    /// [`remove_listener`]: Animation::remove_listener
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener);

    /// Stop calling the listener every time the value of the animation
    /// changes.
    ///
    /// Listeners can be added with [`add_listener`].
    ///
    /// [`add_listener`]: Animation::add_listener
    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener);

    /// Calls listener every time the status of the animation changes.
    ///
    /// Listeners can be removed with [`remove_status_listener`].
    ///
    /// [`remove_status_listener`]: Animation::remove_status_listener
    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener);

    /// Stops calling the listener every time the status of the animation
    /// changes.
    ///
    /// Listeners can be added with [`add_status_listener`].
    ///
    /// [`add_status_listener`]: Animation::add_status_listener
    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    );

    /// The current status of this animation.
    fn status(self: Handle<Self>, app: &App) -> AnimationStatus;

    /// The current value of the animation.
    ///
    /// Returns an owned `T`, not a reference: an animation may compute its
    /// value from its parent's (Dart's `CurvedAnimation.value` does), so there
    /// is no stored field for a reference to point at.
    fn value(self: Handle<Self>, app: &App) -> T;

    /// Whether this animation is stopped at the beginning.
    fn is_dismissed(self: Handle<Self>, app: &App) -> bool {
        Self::status(self, app).is_dismissed()
    }

    /// Whether this animation is stopped at the end.
    fn is_completed(self: Handle<Self>, app: &App) -> bool {
        Self::status(self, app).is_completed()
    }

    /// Whether this animation is running in either direction.
    fn is_animating(self: Handle<Self>, app: &App) -> bool {
        Self::status(self, app).is_animating()
    }

    /// Whether the current aim of this animation is toward completion.
    fn is_forward_or_completed(self: Handle<Self>, app: &App) -> bool {
        Self::status(self, app).is_forward_or_completed()
    }

    /// This animation as the erased [`AnyAnimation<T>`] — what to pass where
    /// a Dart API takes an `Animation<T>`.
    ///
    /// The object is untouched; this mints an erased second handle to it, so
    /// concrete members stay reachable through the typed one while the erased
    /// handle sits in a field Dart types as `Animation<T>`.
    fn as_animation(self: Handle<Self>) -> AnyAnimation<T> {
        AnyAnimation {
            id: self.id(),
            vtable: const { &AnimationVTable::of::<Self>() },
        }
    }
}

/// Erased [`Animation`]: one identity and a static vtable, the fat pointer
/// rustc cannot build for an arena id. No lease.
///
/// This is what a field or parameter Dart types as `Animation<T>` becomes. The
/// concrete animation stays in the [`App`] under its own type; the handle
/// carries its id plus a static dispatch table, so holding or copying one
/// borrows nothing, and the same object stays reachable through its typed
/// [`Handle`] for concrete-only members.
///
/// Equality is Dart's `==` on an object reference: two handles are equal
/// exactly when they address the same object.
pub struct AnyAnimation<T: 'static> {
    id: HandleId,
    vtable: &'static AnimationVTable<T>,
}

/// The vtable of an erased [`AnyAnimation`]: one `&'static` table per
/// concrete [`Animation`] type, built by [`AnimationVTable::of`].
///
/// It lives outside the arena — `&'static` — because a listener member hands
/// `&mut App` onward; dispatch read out of the arena would keep the arena
/// borrowed across that call.
struct AnimationVTable<T> {
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    add_status_listener: fn(&mut App, HandleId, AnimationStatusListener),
    remove_status_listener: fn(&mut App, HandleId, &AnimationStatusListener),
    status: fn(&App, HandleId) -> AnimationStatus,
    value: fn(&App, HandleId) -> T,
    // The four derived getters dispatch through the object rather than
    // recomputing from `status`, because they are virtual in Dart —
    // `AnimationController` overrides `isAnimating`
    // (`animation_controller.dart:449`).
    is_dismissed: fn(&App, HandleId) -> bool,
    is_completed: fn(&App, HandleId) -> bool,
    is_animating: fn(&App, HandleId) -> bool,
    is_forward_or_completed: fn(&App, HandleId) -> bool,
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks
/// the slot.
fn resolve<A: 'static>(id: HandleId) -> Handle<A> {
    Handle::from_id(id)
}

impl<T: 'static> AnimationVTable<T> {
    /// The table for one concrete animation type.
    const fn of<A: Animation<T>>() -> AnimationVTable<T> {
        AnimationVTable {
            add_listener: |app, id, listener| A::add_listener(resolve(id), app, listener),
            remove_listener: |app, id, listener| A::remove_listener(resolve(id), app, listener),
            add_status_listener: |app, id, listener| {
                A::add_status_listener(resolve(id), app, listener)
            },
            remove_status_listener: |app, id, listener| {
                A::remove_status_listener(resolve(id), app, listener)
            },
            status: |app, id| A::status(resolve(id), app),
            value: |app, id| A::value(resolve(id), app),
            is_dismissed: |app, id| A::is_dismissed(resolve(id), app),
            is_completed: |app, id| A::is_completed(resolve(id), app),
            is_animating: |app, id| A::is_animating(resolve(id), app),
            is_forward_or_completed: |app, id| A::is_forward_or_completed(resolve(id), app),
        }
    }
}

impl<T: 'static> AnyAnimation<T> {
    /// Calls the listener every time the value of the animation changes.
    ///
    /// Listeners can be removed with [`remove_listener`](AnyAnimation::remove_listener).
    pub fn add_listener(self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener)
    }

    /// Stop calling the listener every time the value of the animation
    /// changes.
    ///
    /// Listeners can be added with [`add_listener`](AnyAnimation::add_listener).
    pub fn remove_listener(self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener)
    }

    /// Calls listener every time the status of the animation changes.
    ///
    /// Listeners can be removed with
    /// [`remove_status_listener`](AnyAnimation::remove_status_listener).
    pub fn add_status_listener(self, app: &mut App, listener: AnimationStatusListener) {
        (self.vtable.add_status_listener)(app, self.id, listener)
    }

    /// Stops calling the listener every time the status of the animation
    /// changes.
    ///
    /// Listeners can be added with
    /// [`add_status_listener`](AnyAnimation::add_status_listener).
    pub fn remove_status_listener(self, app: &mut App, listener: &AnimationStatusListener) {
        (self.vtable.remove_status_listener)(app, self.id, listener)
    }

    /// The current status of this animation.
    pub fn status(self, app: &App) -> AnimationStatus {
        (self.vtable.status)(app, self.id)
    }

    /// The current value of the animation.
    pub fn value(self, app: &App) -> T {
        (self.vtable.value)(app, self.id)
    }

    /// Whether this animation is stopped at the beginning.
    pub fn is_dismissed(self, app: &App) -> bool {
        (self.vtable.is_dismissed)(app, self.id)
    }

    /// Whether this animation is stopped at the end.
    pub fn is_completed(self, app: &App) -> bool {
        (self.vtable.is_completed)(app, self.id)
    }

    /// Whether this animation is running in either direction.
    pub fn is_animating(self, app: &App) -> bool {
        (self.vtable.is_animating)(app, self.id)
    }

    /// Whether the current aim of this animation is toward completion.
    pub fn is_forward_or_completed(self, app: &App) -> bool {
        (self.vtable.is_forward_or_completed)(app, self.id)
    }
}

impl AnyAnimation<f64> {
    /// Chains a `Tween` (or any [`Animatable`]) to this animation.
    ///
    /// Dart's `Animation<U> drive<U>(Animatable<U> child)` is a generic
    /// virtual method; no dispatch table can hold a generic, so ours is
    /// inherent on the erased handle and cannot be overridden. Flutter defines
    /// no override either — `animation.dart:325` forwards to
    /// `child.animate(this)`, as this does.
    pub fn drive<U: 'static>(
        self,
        app: &mut App,
        child: impl Animatable<U> + Clone + 'static,
    ) -> AnyAnimation<U> {
        child.animate(app, self)
    }
}

impl<T: 'static> reveal_foundation::Listenable for AnyAnimation<T> {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        AnyAnimation::add_listener(*self, app, listener);
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        AnyAnimation::remove_listener(*self, app, listener);
    }
}

// Written by hand so that `AnyAnimation<T>: Copy` does not require `T: Copy`,
// like `Handle<T>`.
impl<T> Clone for AnyAnimation<T> {
    fn clone(&self) -> AnyAnimation<T> {
        *self
    }
}

impl<T> Copy for AnyAnimation<T> {}

impl<T> PartialEq for AnyAnimation<T> {
    fn eq(&self, other: &AnyAnimation<T>) -> bool {
        // The id alone. Dart's `==` on an `Animation` is object identity,
        // which the id carries; the vtable address is excluded because Rust
        // does not promise one address per instantiation across codegen
        // units.
        self.id == other.id
    }
}

impl<T> Eq for AnyAnimation<T> {}

impl<T> Debug for AnyAnimation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyAnimation<{}>({:?})", type_name::<T>(), self.id)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn the_derived_status_getters_match_flutter() {
        use AnimationStatus::*;

        assert_eq!(
            [
                Dismissed.is_dismissed(),
                Forward.is_dismissed(),
                Reverse.is_dismissed(),
                Completed.is_dismissed()
            ],
            [true, false, false, false]
        );
        assert_eq!(
            [
                Dismissed.is_completed(),
                Forward.is_completed(),
                Reverse.is_completed(),
                Completed.is_completed()
            ],
            [false, false, false, true]
        );
        assert_eq!(
            [
                Dismissed.is_animating(),
                Forward.is_animating(),
                Reverse.is_animating(),
                Completed.is_animating()
            ],
            [false, true, true, false]
        );
        assert_eq!(
            [
                Dismissed.is_forward_or_completed(),
                Forward.is_forward_or_completed(),
                Reverse.is_forward_or_completed(),
                Completed.is_forward_or_completed()
            ],
            [false, true, false, true]
        );
    }

    #[test]
    fn only_a_handle_to_the_same_status_closure_compares_equal() {
        let listener = AnimationStatusListener::new(|_status, _app| {});

        assert_eq!(listener, listener.clone());
        assert_ne!(listener, AnimationStatusListener::new(|_status, _app| {}));
    }

    #[test]
    fn a_rebuilt_handle_method_status_listener_matches_like_a_dart_tear_off() {
        struct Host;
        fn hook(_this: Handle<Host>, _app: &mut App, _status: AnimationStatus) {}

        let mut app = App::new();
        let a = app.create(Host);
        let b = app.create(Host);

        assert_eq!(
            AnimationStatusListener::handle_method(a, hook),
            AnimationStatusListener::handle_method(a, hook)
        );
        assert_ne!(
            AnimationStatusListener::handle_method(a, hook),
            AnimationStatusListener::handle_method(b, hook)
        );
        assert_ne!(
            AnimationStatusListener::handle_method(a, hook),
            AnimationStatusListener::new(|_status, _app| {})
        );
    }

    /// A minimal node: the erased handle must reach the overriding impl, not
    /// the trait defaults — `AnimationController` overrides `isAnimating`
    /// while stopped (`animation_controller.dart:449`).
    struct OverridingNode;

    impl Animation<f64> for OverridingNode {
        fn add_listener(self: Handle<Self>, _app: &mut App, _listener: Listener) {}
        fn remove_listener(self: Handle<Self>, _app: &mut App, _listener: &Listener) {}
        fn add_status_listener(
            self: Handle<Self>,
            _app: &mut App,
            _listener: AnimationStatusListener,
        ) {
        }
        fn remove_status_listener(
            self: Handle<Self>,
            _app: &mut App,
            _listener: &AnimationStatusListener,
        ) {
        }
        fn status(self: Handle<Self>, _app: &App) -> AnimationStatus {
            AnimationStatus::Forward
        }
        fn value(self: Handle<Self>, _app: &App) -> f64 {
            0.25
        }
        fn is_animating(self: Handle<Self>, _app: &App) -> bool {
            // Forward would say true; the override says false.
            false
        }
    }

    #[test]
    fn the_erased_handle_dispatches_overridden_getters() {
        let mut app = App::new();
        let node = app.create(OverridingNode);
        let animation = node.as_animation();

        assert_eq!(animation.status(&app), AnimationStatus::Forward);
        assert_eq!(animation.value(&app), 0.25);
        assert!(
            !animation.is_animating(&app),
            "the override, not the status"
        );
        assert!(animation.is_forward_or_completed(&app), "the default");
    }

    #[test]
    fn erased_handles_are_equal_exactly_when_their_entities_are() {
        let mut app = App::new();
        let first = app.create(OverridingNode);
        let second = app.create(OverridingNode);

        assert_eq!(first.as_animation(), first.as_animation());
        assert_ne!(first.as_animation(), second.as_animation());
    }

    /// A node whose `value` reads its slot.
    struct Counter(u32);

    impl Animation<f64> for Counter {
        fn add_listener(self: Handle<Self>, _app: &mut App, _listener: Listener) {}
        fn remove_listener(self: Handle<Self>, _app: &mut App, _listener: &Listener) {}
        fn add_status_listener(
            self: Handle<Self>,
            _app: &mut App,
            _listener: AnimationStatusListener,
        ) {
        }
        fn remove_status_listener(
            self: Handle<Self>,
            _app: &mut App,
            _listener: &AnimationStatusListener,
        ) {
        }
        fn status(self: Handle<Self>, _app: &App) -> AnimationStatus {
            AnimationStatus::Forward
        }
        fn value(self: Handle<Self>, app: &App) -> f64 {
            f64::from(app.get(self).0)
        }
    }

    #[test]
    fn an_erased_handle_does_not_consume_the_typed_one() {
        let mut app = App::new();
        let counter = app.create(Counter(1));
        let erased = counter.as_animation();

        // The concrete member stays reachable after erasing — the failure that
        // killed the owned `Box<dyn>` shape.
        app.get_mut(counter).0 = 2;
        assert_eq!(erased.value(&app), 2.0);
    }

    /// The erased handle looks nothing up; the stale check is `App::get`'s,
    /// at the first member that reads the slot.
    #[test]
    #[should_panic(expected = "stale handle")]
    fn an_erased_handle_to_a_destroyed_animation_panics_on_slot_access() {
        let mut app = App::new();
        let counter = app.create(Counter(1));
        let animation = counter.as_animation();
        app.destroy(counter);
        animation.value(&app);
    }

    #[test]
    fn a_status_listener_receives_the_status_and_the_app() {
        thread_local! {
            static RECEIVED: Cell<Option<AnimationStatus>> = const { Cell::new(None) };
        }
        let listener = AnimationStatusListener::new(|status, _app| {
            RECEIVED.with(|received| received.set(Some(status)));
        });
        listener.call(AnimationStatus::Reverse, &mut App::new());
        assert_eq!(RECEIVED.with(Cell::get), Some(AnimationStatus::Reverse));
    }
}
