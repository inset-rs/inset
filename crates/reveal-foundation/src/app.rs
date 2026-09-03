//! The one [`App`] and the arena of [`Handle`]s inside it. Not a Dart file.

use std::any::{Any, type_name};
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{InertPlatform, PlatformRef};
use slotmap::{SlotMap, new_key_type};

use crate::change_notifier::Listener;
use crate::timers::{Timer, Timers};

/// Drain budget for one [`App::drain_microtasks`]: two callbacks scheduling
/// each other forever must fail loud, not starve the event loop. Not a Dart
/// or Flutter constant — the isolate drains until empty; a cycle there hangs.
const MICROTASK_BUDGET: usize = 100_000;

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
#[non_exhaustive]
pub struct App {
    slots: SlotMap<HandleId, Slot>,
    singletons: HashMap<std::any::TypeId, HandleId>,
    microtasks: VecDeque<Listener>,
    timers: Timers,
    platform: PlatformRef,
}

impl Default for App {
    fn default() -> App {
        App {
            slots: SlotMap::with_key(),
            singletons: HashMap::new(),
            microtasks: VecDeque::new(),
            timers: Timers::default(),
            platform: Rc::new(InertPlatform),
        }
    }
}

impl App {
    pub fn new() -> App {
        App::default()
    }

    /// The start closure (or tests that need a live engine) builds the
    /// platform first, then this. [`App::new`] leaves an inert platform
    /// and no views; tests that pump frames themselves use that.
    pub fn with_platform(platform: PlatformRef) -> App {
        App {
            platform,
            ..App::default()
        }
    }

    pub fn platform(&self) -> PlatformRef {
        Rc::clone(&self.platform)
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

    /// Dart's `scheduleMicrotask`: queues `callback` to run after the current
    /// turn, before the next platform event — never inline.
    ///
    /// The host empties the queue with
    /// [`drain_microtasks`](App::drain_microtasks) after every platform
    /// event, and between `_beginFrame` and `_drawFrame` — where the engine,
    /// which runs the two in one native task, puts an explicit flush.
    pub fn schedule_microtask(&mut self, callback: Listener) {
        self.microtasks.push_back(callback);
    }

    /// Runs queued microtasks until the queue is empty.
    ///
    /// A callback scheduled during the drain runs in the same drain, as
    /// Dart's queue does. Call with no borrow of the App held.
    ///
    /// # Panics
    ///
    /// After [`MICROTASK_BUDGET`] callbacks in one drain, on the assumption
    /// that two callbacks are scheduling each other in a cycle.
    pub fn drain_microtasks(&mut self) {
        let mut drained = 0usize;
        while let Some(callback) = self.microtasks.pop_front() {
            drained += 1;
            assert!(
                drained <= MICROTASK_BUDGET,
                "microtasks did not converge after {MICROTASK_BUDGET} callbacks; \
                 are two callbacks scheduling each other in a cycle?"
            );
            callback.call(self);
        }
    }

    /// Whether any microtask is queued.
    ///
    /// The queue is empty when a new platform event begins, and the engine
    /// explicitly flushes it between the two frame callbacks; the scheduler
    /// asserts the latter.
    pub fn has_pending_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    /// Dart's `Timer(duration, callback)`. Fires when the clock reaches
    /// `now + duration` via [`elapse`](Self::elapse).
    pub fn schedule_timer(&mut self, duration: Duration, callback: Listener) -> Timer {
        let (timer, wake) = self.timers.schedule(duration, callback);
        if let Some(delay) = wake {
            self.platform.wake_at(self.platform.now() + delay);
        }
        timer
    }

    /// Dart's `Timer.cancel()`. Idempotent.
    pub fn cancel_timer(&mut self, timer: Timer) {
        self.timers.cancel(timer);
    }

    /// Dart's `Timer.isActive`.
    pub fn timer_is_active(&self, timer: Timer) -> bool {
        self.timers.is_active(timer)
    }

    /// Advances the App clock by `duration` and fires due timers.
    ///
    /// Microtasks drain first, then each due timer in due order (ties by id),
    /// then microtasks after each callback — Dart's event-loop position for
    /// `Timer`. Tests call this (FakeAsync). A host calls it as time passes.
    pub fn elapse(&mut self, duration: Duration) {
        let target = self.timers.now() + duration;
        self.drain_microtasks();
        let mut fired = 0usize;
        loop {
            let Some(callback) = self.timers.pop_next_due(target) else {
                break;
            };
            fired += 1;
            Timers::assert_fire_budget(fired);
            callback.call(self);
            self.drain_microtasks();
        }
        self.timers.advance_to(target);
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
    use crate::Timer;

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

    #[test]
    fn a_microtask_runs_on_drain_not_inline() {
        let mut app = App::new();
        let counter = app.create(Counter(0));
        app.schedule_microtask(crate::Listener::new(move |app| {
            app.get_mut(counter).0 = 1;
        }));
        assert_eq!(app.get(counter).0, 0);
        assert!(app.has_pending_microtasks());
        app.drain_microtasks();
        assert_eq!(app.get(counter).0, 1);
        assert!(!app.has_pending_microtasks());
    }

    #[test]
    fn a_microtask_scheduled_during_drain_runs_in_the_same_drain() {
        let mut app = App::new();
        let counter = app.create(Counter(0));
        app.schedule_microtask(crate::Listener::new(move |app| {
            app.get_mut(counter).0 = 1;
            app.schedule_microtask(crate::Listener::new(move |app| {
                app.get_mut(counter).0 = 2;
            }));
        }));
        app.drain_microtasks();
        assert_eq!(app.get(counter).0, 2);
    }

    #[test]
    fn timers_fire_in_due_order_after_microtasks() {
        let mut app = App::new();
        let log = app.create(Vec::<&'static str>::new());
        app.schedule_microtask(crate::Listener::new(move |app| {
            app.get_mut(log).push("micro");
        }));
        Timer::new(
            &mut app,
            Duration::from_millis(200),
            crate::Listener::new(move |app| {
                app.get_mut(log).push("late");
            }),
        );
        Timer::new(
            &mut app,
            Duration::from_millis(100),
            crate::Listener::new(move |app| {
                app.get_mut(log).push("early");
            }),
        );
        Timer::new(
            &mut app,
            Duration::from_millis(100),
            crate::Listener::new(move |app| {
                app.get_mut(log).push("early-second");
            }),
        );
        assert!(app.get(log).is_empty());
        app.elapse(Duration::from_millis(200));
        assert_eq!(
            app.get(log).as_slice(),
            ["micro", "early", "early-second", "late"]
        );
    }

    #[test]
    fn cancel_prevents_a_timer_from_firing() {
        let mut app = App::new();
        let counter = app.create(Counter(0));
        let timer = Timer::new(
            &mut app,
            Duration::from_millis(10),
            crate::Listener::new(move |app| {
                app.get_mut(counter).0 = 1;
            }),
        );
        assert!(timer.is_active(&app));
        timer.cancel(&mut app);
        assert!(!timer.is_active(&app));
        app.elapse(Duration::from_millis(10));
        assert_eq!(app.get(counter).0, 0);
    }
}
