//! The one [`App`] and the arena of [`Handle`]s inside it. Not a Dart file.

use std::any::{Any, type_name};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use slotmap::{SlotMap, new_key_type};

new_key_type! {
    /// Untyped [`Handle`]. For an edge that may point at more than one type; [`App::handle`] narrows it.
    ///
    /// Destroy bumps the generation, so a leftover id does not silently name the next occupant.
    pub struct HandleId;
}

/// A Dart object, as far as Rust ownership is concerned.
///
/// Flutter objects form cycles — an element names its widget and its render object, a notifier
/// names its listeners, a listener often names the notifier. Dart allows that because the GC
/// owns the objects. Here the object lives in [`App`]; what you store is this id.
///
/// `Copy` and owns nothing. A parent–child edge is a field, not an `Rc`. Do not add a refcount:
/// that would drop `Copy` and is the `Entity` design, reserved for user stores. Stale is
/// detected by generation, not by keeping the slot alive.
///
/// Point access, not a lease: [`App::get`] / [`App::get_mut`] for one field, then drop the borrow
/// before any call that can run framework code. Holding the object out of the arena for a whole
/// pass (shaft-rs-next) meant a layout callback could not re-enter the node it was laying out.
///
/// [`App::get`] panics on stale — a typed handle is a promise the slot is live. [`App::handle`]
/// returns [`None`] for an id that might be dead or the wrong type.
///
/// `PhantomData<fn() -> T>` so `Handle<T>: Copy` even when `T` is not.
pub struct Handle<T> {
    id: HandleId,
    state: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    /// Unchecked. Only [`App::create`] mints these; [`App::handle`] is the checked path.
    fn new(id: HandleId) -> Handle<T> {
        Handle {
            id,
            state: PhantomData,
        }
    }

    pub fn id(self) -> HandleId {
        self.id
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Handle<T> {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Handle<T>) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle<{}>({:?})", type_name::<T>(), self.id)
    }
}

impl<T> From<Handle<T>> for HandleId {
    fn from(handle: Handle<T>) -> HandleId {
        handle.id
    }
}

struct Slot {
    type_name: &'static str,
    state: Box<dyn Any>,
}

/// Owns every Flutter object. Callbacks receive `&mut App` plus a [`Handle`] to themselves.
///
/// Private and `#[non_exhaustive]`: scheduler, timers, and further arenas are additive.
#[derive(Default)]
#[non_exhaustive]
pub struct App {
    slots: SlotMap<HandleId, Slot>,
    singletons: HashMap<std::any::TypeId, HandleId>,
}

impl App {
    pub fn new() -> App {
        App::default()
    }

    /// One `T` per App (`SchedulerBinding.instance`, `kAlwaysCompleteAnimation`). Do not destroy it — the stored id goes stale and the next call panics.
    pub fn singleton<T: 'static + Default>(&mut self) -> Handle<T> {
        if let Some(&id) = self.singletons.get(&std::any::TypeId::of::<T>()) {
            return Handle::new(id);
        }
        let handle = self.create(T::default());
        self.singletons
            .insert(std::any::TypeId::of::<T>(), handle.id());
        handle
    }

    pub fn create<T: 'static>(&mut self, state: T) -> Handle<T> {
        Handle::new(self.slots.insert(Slot {
            type_name: type_name::<T>(),
            state: Box::new(state),
        }))
    }

    /// Vacates the slot. Later [`get`](App::get) through this id panics.
    ///
    /// # Panics
    ///
    /// If already destroyed.
    pub fn destroy(&mut self, handle: impl Into<HandleId>) {
        let id = handle.into();
        assert!(
            self.slots.remove(id).is_some(),
            "stale handle: {id:?} was already destroyed"
        );
    }

    pub fn contains(&self, handle: impl Into<HandleId>) -> bool {
        self.slots.contains_key(handle.into())
    }

    /// Narrow a [`HandleId`]. `None` if stale or the wrong type. [`get`](App::get) panics instead.
    pub fn handle<T: 'static>(&self, handle: impl Into<HandleId>) -> Option<Handle<T>> {
        let id = handle.into();
        if !self.slots.get(id)?.state.is::<T>() {
            return None;
        }
        Some(Handle::new(id))
    }

    /// # Panics
    ///
    /// If the handle is stale.
    pub fn get<T: 'static>(&self, handle: Handle<T>) -> &T {
        let slot = self.resolve(handle.id);
        slot.state.downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "handle {:?} holds `{}`, not `{}`",
                handle.id,
                slot.type_name,
                type_name::<T>()
            )
        })
    }

    /// See [`get`](App::get).
    pub fn get_mut<T: 'static>(&mut self, handle: Handle<T>) -> &mut T {
        let id = handle.id;
        let slot = self.resolve_mut(id);
        let type_name_in_slot = slot.type_name;
        slot.state.downcast_mut::<T>().unwrap_or_else(|| {
            panic!(
                "handle {id:?} holds `{type_name_in_slot}`, not `{}`",
                type_name::<T>()
            )
        })
    }

    fn resolve(&self, id: HandleId) -> &Slot {
        self.slots
            .get(id)
            .unwrap_or_else(|| panic!("stale handle: {id:?} was destroyed"))
    }

    fn resolve_mut(&mut self, id: HandleId) -> &mut Slot {
        self.slots
            .get_mut(id)
            .unwrap_or_else(|| panic!("stale handle: {id:?} was destroyed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq)]
    struct Counter(i32);

    #[derive(Debug, PartialEq)]
    struct Label(&'static str);

    #[test]
    fn state_survives_a_round_trip_through_a_handle() {
        let mut app = App::new();
        let counter = app.create(Counter(1));

        assert_eq!(app.get(counter), &Counter(1));
        app.get_mut(counter).0 = 7;
        assert_eq!(app.get(counter), &Counter(7));
    }

    #[test]
    fn handles_of_different_types_share_one_arena() {
        let mut app = App::new();
        let counter = app.create(Counter(1));
        let label = app.create(Label("hello"));

        assert_eq!(app.get(counter), &Counter(1));
        assert_eq!(app.get(label), &Label("hello"));
    }

    #[test]
    fn a_destroyed_slot_is_reused_but_its_handle_is_not() {
        let mut app = App::new();
        let first = app.create(Counter(1));
        let first_id = first.id();
        app.destroy(first);

        let second = app.create(Counter(2));
        // slotmap: version in high 32 bits, index in low 32. Without reuse this test is vacuous.
        let slot_index = |id: HandleId| slotmap::Key::data(&id).as_ffi() & 0xffff_ffff;
        assert_eq!(
            slot_index(second.id()),
            slot_index(first_id),
            "the vacated slot is reused, so the version is what tells them apart"
        );
        assert_ne!(second.id(), first_id);
        assert!(!app.contains(first_id));
        assert!(app.contains(second));
    }

    #[test]
    #[should_panic(expected = "stale handle")]
    fn reading_through_a_stale_handle_panics() {
        let mut app = App::new();
        let counter = app.create(Counter(1));
        app.destroy(counter);
        app.get(counter);
    }

    #[test]
    fn the_checked_path_from_an_untyped_id_answers_none_rather_than_panicking() {
        let mut app = App::new();
        let counter = app.create(Counter(1));
        let id = counter.id();

        assert_eq!(app.handle::<Counter>(id), Some(counter));
        assert_eq!(app.handle::<Label>(id), None, "wrong type");

        app.destroy(counter);
        assert_eq!(app.handle::<Counter>(id), None, "stale");
    }

    #[test]
    fn a_singleton_is_one_handle_per_type_per_app() {
        let mut app = App::new();

        let first: Handle<Counter> = app.singleton();
        let second: Handle<Counter> = app.singleton();
        assert_eq!(first, second);

        app.get_mut(first).0 = 9;
        assert_eq!(app.get(second).0, 9, "one slot behind both handles");

        let mut other_app = App::new();
        let elsewhere: Handle<Counter> = other_app.singleton();
        assert_eq!(elsewhere.id(), first.id(), "ids may collide across Apps");
        assert_eq!(other_app.get(elsewhere).0, 0, "but the state is per App");
    }

    #[test]
    fn a_handle_is_copy_so_an_edge_can_be_a_plain_field() {
        let mut app = App::new();
        let counter = app.create(Counter(1));

        struct Edge {
            target: Handle<Counter>,
        }
        let edge = Edge { target: counter };
        let copied = edge.target;

        assert_eq!(copied, counter);
        assert_eq!(app.get(edge.target).0, 1);
    }
}
