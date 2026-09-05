//! The one [`App`] and the arena of [`Handle`]s inside it. Not a Dart file.

use std::any::{Any, type_name};
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::Receiver;
use std::rc::{Rc, Weak};
use std::time::Duration;

use reveal_embedder::{InertPlatform, PlatformRef};
use slotmap::{SlotMap, new_key_type};

use crate::app_cell::{AppCell, AsyncApp};
use crate::change_notifier::Listener;
use crate::executor::{ForegroundExecutor, Task};
use crate::timers::{Timer, Timers};

/// Drain budget for one [`App::drain_microtasks`]: two callbacks scheduling
/// each other forever must fail loud, not starve the event loop. Not a Dart
/// or Flutter constant — the isolate drains until empty; a cycle there hangs.
const MICROTASK_BUDGET: usize = 100_000;

new_key_type! {
    /// Untyped [`Handle`]. For a reference that may point at more than one type;
    /// [`App::handle`] narrows it.
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
    /// Unchecked. [`App::create`] mints these; [`App::handle`] is the checked path and
    /// [`from_id`](Self::from_id) the trusted one.
    fn new(id: HandleId) -> Handle<T> {
        Handle {
            id,
            state: PhantomData,
        }
    }

    /// Wraps an id that was minted for `T` without looking at the slot. The check is deferred to
    /// [`App::get`], which panics on a stale or wrong-typed id. For a type-erased handle that
    /// reconstructs the typed handle it was made from.
    pub fn from_id(id: HandleId) -> Handle<T> {
        Handle::new(id)
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

/// An arena object's methods take `self: Handle<Self>`: the handle is the receiver, the
/// object is `app.get(self)`.
impl<T> Receiver for Handle<T> {
    type Target = T;
}

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
/// Lives in an [`AppCell`], which is what the shell and the tests hold; a task reaches it
/// through [`AsyncApp`]. Private and `#[non_exhaustive]`: scheduler, timers, and further
/// arenas are additive.
#[non_exhaustive]
pub struct App {
    /// The cell this App lives in — gpui's `App::this`. Dangling for a bare [`App::new`].
    this: Weak<AppCell>,
    slots: SlotMap<HandleId, Slot>,
    singletons: HashMap<std::any::TypeId, HandleId>,
    microtasks: VecDeque<Listener>,
    timers: Timers,
    executor: ForegroundExecutor,
    platform: PlatformRef,
}

/// A bare `App` outside any cell: see [`App::new`].
impl Default for App {
    fn default() -> App {
        App::build(
            Weak::new(),
            Rc::new(InertPlatform),
            ForegroundExecutor::new(),
        )
    }
}

impl App {
    /// A bare `App` outside any [`AppCell`]: it cannot [`spawn`](Self::spawn), and its
    /// [`elapse`](Self::elapse) resumes no task. Kept for the tests written before the cell;
    /// new code builds through [`AppCell::new`].
    pub fn new() -> App {
        App::default()
    }

    /// A bare `App` on a live platform; see [`new`](Self::new). New code builds through
    /// [`AppCell::with_platform`].
    pub fn with_platform(platform: PlatformRef) -> App {
        App {
            platform,
            ..App::default()
        }
    }

    pub(crate) fn build(
        this: Weak<AppCell>,
        platform: PlatformRef,
        executor: ForegroundExecutor,
    ) -> App {
        App {
            this,
            slots: SlotMap::with_key(),
            singletons: HashMap::new(),
            microtasks: VecDeque::new(),
            timers: Timers::default(),
            executor,
            platform,
        }
    }

    /// This App as a task sees it: the handle an `async` body captures across its `await`s.
    pub fn to_async(&self) -> AsyncApp {
        AsyncApp::new(self.this.clone())
    }

    /// Queues `f` as a task on this App: the continuation of a Dart `async` body.
    ///
    /// Dart runs an `async` body synchronously up to its first `await`; port that prefix
    /// inline — including the call whose future is awaited — and spawn only what follows the
    /// `await`. The continuation first runs at the next [`AppCell::checkpoint`], the end of
    /// the current platform event, and thereafter at the checkpoint after whatever it awaited
    /// completed; it cannot run inline, since the caller holds the App it would borrow.
    ///
    /// The returned [`Task`] is the continuation's future; dropping it does not cancel it.
    ///
    /// # Panics
    ///
    /// On a bare [`App::new`], which no cell can drain.
    pub fn spawn<R: 'static>(&self, f: impl AsyncFnOnce(&mut AsyncApp) -> R + 'static) -> Task<R> {
        assert!(
            self.this.strong_count() > 0,
            "App::spawn on an App outside an AppCell: build it with AppCell::new"
        );
        let mut cx = self.to_async();
        self.executor.spawn(async move { f(&mut cx).await })
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
    /// After `MICROTASK_BUDGET` callbacks in one drain, on the assumption
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

    /// Advances the App clock by `duration` and fires due timers, on a bare [`App::new`].
    ///
    /// [`AppCell::elapse`] is the one that lets a task resume between timers; this keeps the
    /// same order (microtasks first, then each due timer, microtasks after each) for the tests
    /// written before the cell.
    pub fn elapse(&mut self, duration: Duration) {
        let target = self.clock() + duration;
        self.drain_microtasks();
        let mut fired = 0usize;
        while self.fire_next_due(target) {
            fired += 1;
            Timers::assert_fire_budget(fired);
            self.drain_microtasks();
        }
        self.advance_clock_to(target);
    }

    /// The App clock: what [`elapse`](Self::elapse) has advanced it to.
    pub(crate) fn clock(&self) -> Duration {
        self.timers.now()
    }

    /// Fires the next timer due by `target`, if any.
    pub(crate) fn fire_next_due(&mut self, target: Duration) -> bool {
        let Some(callback) = self.timers.pop_next_due(target) else {
            return false;
        };
        callback.call(self);
        true
    }

    pub(crate) fn advance_clock_to(&mut self, target: Duration) {
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
        downcast_slot_mut(slot, id)
    }

    /// Two live slots at once, for an object that works on another (a render object shaping
    /// its text against the font collection).
    ///
    /// # Panics
    ///
    /// If either handle is stale, or both name the same slot.
    pub fn get_disjoint_mut<A: 'static, B: 'static>(
        &mut self,
        a: Handle<A>,
        b: Handle<B>,
    ) -> (&mut A, &mut B) {
        let [slot_a, slot_b] = self
            .slots
            .get_disjoint_mut([a.id, b.id])
            .unwrap_or_else(|| panic!("handles {:?} and {:?} are not two live slots", a.id, b.id));
        (
            downcast_slot_mut(slot_a, a.id),
            downcast_slot_mut(slot_b, b.id),
        )
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

fn downcast_slot_mut<T: 'static>(slot: &mut Slot, id: HandleId) -> &mut T {
    let type_name_in_slot = slot.type_name;
    slot.state.downcast_mut::<T>().unwrap_or_else(|| {
        panic!(
            "handle {id:?} holds `{type_name_in_slot}`, not `{}`",
            type_name::<T>()
        )
    })
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
    #[test]
    fn get_disjoint_mut_borrows_two_slots_at_once() {
        let mut app = App::new();
        let a = app.create(1u32);
        let b = app.create(String::from("x"));
        let (a_value, b_value) = app.get_disjoint_mut(a, b);
        *a_value += 1;
        b_value.push('y');
        assert_eq!(*app.get(a), 2);
        assert_eq!(app.get(b), "xy");
    }

    #[test]
    #[should_panic(expected = "not two live slots")]
    fn get_disjoint_mut_rejects_the_same_slot_twice() {
        let mut app = App::new();
        let a = app.create(1u32);
        let _ = app.get_disjoint_mut(a, a);
    }

    #[test]
    fn from_id_reads_like_the_minted_handle() {
        let mut app = App::new();
        let minted = app.create(Counter(7));
        let rebuilt = Handle::<Counter>::from_id(minted.id());
        assert_eq!(rebuilt, minted);
        assert_eq!(app.get(rebuilt).0, 7);
    }

    #[test]
    #[should_panic(expected = "stale handle")]
    fn from_id_defers_the_stale_check_to_get() {
        let mut app = App::new();
        let minted = app.create(Counter(0));
        let id = minted.id();
        app.destroy(minted);
        let rebuilt = Handle::<Counter>::from_id(id);
        app.get(rebuilt);
    }
}
