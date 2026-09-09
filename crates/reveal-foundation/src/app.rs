//! The one [`App`] and the arena of [`Handle`]s inside it. Not a Dart file.

use std::any::type_name;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::Receiver;
use std::rc::{Rc, Weak};
use std::time::Duration;

use reveal_embedder::{PlatformRef, ViewFocusEvent};
use slotmap::new_key_type;

use crate::app_cell::{AppCell, AsyncApp};
use crate::change_notifier::Listener;
use crate::executor::{ForegroundExecutor, Task};
use crate::handle_map::HandleMap;
pub use crate::handle_map::{RetainedHandle, RetainedHandleId};
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
/// detected by generation, not by keeping the entry alive.
///
/// Point access, not a lease: [`App::get`] / [`App::get_mut`] for one field, then drop the borrow
/// before any call that can run framework code. Holding the object out of the arena for a whole
/// pass (shaft-rs-next) meant a layout callback could not re-enter the node it was laying out.
///
/// [`App::get`] panics on stale — a typed handle is a promise the entry is live. [`App::handle`]
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
    pub(crate) fn new(id: HandleId) -> Handle<T> {
        Handle {
            id,
            state: PhantomData,
        }
    }

    /// Wraps an id that was minted for `T` without looking at the entry. The check is deferred to
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

/// Owns every Flutter object. Callbacks receive `&mut App` plus a [`Handle`] to themselves.
///
/// Lives in an [`AppCell`], which is what the shell and the tests hold; a task reaches it
/// through [`AsyncApp`]. Private and `#[non_exhaustive]`: scheduler, timers, and further
/// arenas are additive.
#[non_exhaustive]
pub struct App {
    /// The cell this App lives in — gpui's `App::this`.
    this: Weak<AppCell>,
    handles: HandleMap,
    singletons: HashMap<std::any::TypeId, HandleId>,
    microtasks: VecDeque<Listener>,
    timers: Timers,
    executor: ForegroundExecutor,
    platform: PlatformRef,
    platform_callbacks: PlatformCallbacks,
}

/// The callbacks the framework assigns onto dart:ui's `PlatformDispatcher`
/// (`onPlatformBrightnessChanged`, `onLocaleChanged`). The host's half of the dispatcher
/// is the `Platform` the `App` holds; this is the half the framework sets and the host's
/// client invokes.
#[derive(Default)]
pub struct PlatformCallbacks {
    pub on_platform_brightness_changed: Option<Listener>,
    pub on_locale_changed: Option<Listener>,
    /// Flutter `PlatformDispatcher.onViewFocusChange`.
    pub on_view_focus_change: Option<Rc<dyn Fn(&mut App, ViewFocusEvent)>>,
}

impl App {
    pub(crate) fn build(
        this: Weak<AppCell>,
        platform: PlatformRef,
        executor: ForegroundExecutor,
    ) -> App {
        App {
            this,
            handles: HandleMap::new(),
            singletons: HashMap::new(),
            microtasks: VecDeque::new(),
            timers: Timers::default(),
            executor,
            platform,
            platform_callbacks: PlatformCallbacks::default(),
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
    pub fn spawn<R: 'static>(&self, f: impl AsyncFnOnce(&mut AsyncApp) -> R + 'static) -> Task<R> {
        let mut cx = self.to_async();
        self.executor.spawn(async move { f(&mut cx).await })
    }

    pub fn platform(&self) -> PlatformRef {
        Rc::clone(&self.platform)
    }

    /// The dispatcher callbacks the framework has assigned.
    pub fn platform_callbacks(&self) -> &PlatformCallbacks {
        &self.platform_callbacks
    }

    pub fn platform_callbacks_mut(&mut self) -> &mut PlatformCallbacks {
        &mut self.platform_callbacks
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
    /// `now + duration` via [`AppCell::elapse`].
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

    /// The App clock: what [`AppCell::elapse`] has advanced it to.
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

    /// Asks the platform to wake for the earliest pending timer. Called once the timers due by
    /// now have fired: the deadline the platform held was theirs, and the ones still queued
    /// (scheduled while an earlier timer was pending, or by the callbacks that just ran) need one.
    pub(crate) fn request_wake_for_next_timer(&mut self) {
        if let Some(delay) = self.timers.next_wake() {
            self.platform.wake_at(self.platform.now() + delay);
        }
    }

    pub fn create<T: 'static>(&mut self, state: T) -> Handle<T> {
        self.handles.create(state)
    }

    /// Dart's `dispose`: see [`HandleMap::destroy`].
    pub fn destroy(&mut self, handle: impl Into<HandleId>) {
        self.handles.destroy(handle.into());
    }

    /// Whether the entry exists: live, or destroyed but still retained.
    pub fn contains(&self, handle: impl Into<HandleId>) -> bool {
        self.handles.contains(handle.into())
    }

    /// A [`RetainedHandle`] that keeps the object past its owner's `destroy`; see
    /// [`HandleMap::retain`]. `T` is the Copy handle to keep (`Handle<U>` or an erased handle).
    pub fn retain<T: Copy + Into<HandleId>>(&mut self, handle: T) -> RetainedHandle<T> {
        self.handles.retain(handle)
    }

    /// A [`RetainedHandleId`] when the holder has only the id; see [`HandleMap::retain_id`].
    pub fn retain_id(&mut self, id: HandleId) -> RetainedHandleId {
        self.handles.retain_id(id)
    }

    /// Gives a [`RetainedHandle`] back now; a dropped one is released at the next checkpoint.
    pub fn release<T>(&mut self, handle: RetainedHandle<T>) {
        self.handles.release(handle);
    }

    /// See [`release`](Self::release).
    pub fn release_id(&mut self, handle: RetainedHandleId) {
        self.handles.release_id(handle);
    }

    pub(crate) fn release_dropped_retained_handles(&mut self) {
        self.handles.release_dropped_retained_handles();
    }

    /// Whether the owner destroyed the object while a [`RetainedHandle`] still keeps it.
    pub fn is_disposed(&self, handle: impl Into<HandleId>) -> bool {
        self.handles.is_disposed(handle.into())
    }

    /// Narrow a [`HandleId`]. `None` if stale or the wrong type. [`get`](App::get) panics instead.
    pub fn handle<T: 'static>(&self, handle: impl Into<HandleId>) -> Option<Handle<T>> {
        self.handles.handle(handle.into())
    }

    /// # Panics
    ///
    /// If the handle is stale.
    pub fn get<T: 'static>(&self, handle: Handle<T>) -> &T {
        self.handles.get(handle)
    }

    /// See [`get`](App::get).
    pub fn get_mut<T: 'static>(&mut self, handle: Handle<T>) -> &mut T {
        self.handles.get_mut(handle)
    }

    /// Two live entries at once, for an object that works on another (a render object shaping
    /// its text against the font collection).
    ///
    /// # Panics
    ///
    /// If either handle is stale, or both name the same entry.
    pub fn get_disjoint_mut<A: 'static, B: 'static>(
        &mut self,
        a: Handle<A>,
        b: Handle<B>,
    ) -> (&mut A, &mut B) {
        self.handles.get_disjoint_mut(a, b)
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.create(Counter(1));

        assert_eq!(app.get(counter), &Counter(1));
        app.get_mut(counter).0 = 7;
        assert_eq!(app.get(counter), &Counter(7));
    }

    #[test]
    fn handles_of_different_types_share_one_arena() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.create(Counter(1));
        let label = app.create(Label("hello"));

        assert_eq!(app.get(counter), &Counter(1));
        assert_eq!(app.get(label), &Label("hello"));
    }

    #[test]
    fn a_destroyed_entry_is_reused_but_its_handle_is_not() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = app.create(Counter(1));
        let first_id = first.id();
        app.destroy(first);

        let second = app.create(Counter(2));
        // slotmap: version in high 32 bits, index in low 32. Without reuse this test is vacuous.
        let entry_index = |id: HandleId| slotmap::Key::data(&id).as_ffi() & 0xffff_ffff;
        assert_eq!(
            entry_index(second.id()),
            entry_index(first_id),
            "the freed entry is reused, so the version is what tells them apart"
        );
        assert_ne!(second.id(), first_id);
        assert!(!app.contains(first_id));
        assert!(app.contains(second));
    }

    #[test]
    #[should_panic(expected = "stale handle")]
    fn reading_through_a_stale_handle_panics() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.create(Counter(1));
        app.destroy(counter);
        app.get(counter);
    }

    #[test]
    fn the_checked_path_from_an_untyped_id_answers_none_rather_than_panicking() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.create(Counter(1));
        let id = counter.id();

        assert_eq!(app.handle::<Counter>(id), Some(counter));
        assert_eq!(app.handle::<Label>(id), None, "wrong type");

        app.destroy(counter);
        assert_eq!(app.handle::<Counter>(id), None, "stale");
    }

    #[test]
    fn a_singleton_is_one_handle_per_type_per_app() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();

        let first: Handle<Counter> = app.singleton();
        let second: Handle<Counter> = app.singleton();
        assert_eq!(first, second);

        app.get_mut(first).0 = 9;
        assert_eq!(app.get(second).0, 9, "one entry behind both handles");

        let other_cell = AppCell::new();
        let mut other_app = other_cell.borrow_mut();
        let elsewhere: Handle<Counter> = other_app.singleton();
        assert_eq!(elsewhere.id(), first.id(), "ids may collide across Apps");
        assert_eq!(other_app.get(elsewhere).0, 0, "but the state is per App");
    }

    #[test]
    fn a_handle_is_copy_so_an_edge_can_be_a_plain_field() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        drop(app);
        cell.elapse(Duration::from_millis(200));
        let app = cell.borrow();
        assert_eq!(
            app.get(log).as_slice(),
            ["micro", "early", "early-second", "late"]
        );
    }

    #[test]
    fn cancel_prevents_a_timer_from_firing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        drop(app);
        cell.elapse(Duration::from_millis(10));
        let app = cell.borrow();
        assert_eq!(app.get(counter).0, 0);
    }
    #[test]
    fn get_disjoint_mut_borrows_two_entries_at_once() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let a = app.create(1u32);
        let b = app.create(String::from("x"));
        let (a_value, b_value) = app.get_disjoint_mut(a, b);
        *a_value += 1;
        b_value.push('y');
        assert_eq!(*app.get(a), 2);
        assert_eq!(app.get(b), "xy");
    }

    #[test]
    #[should_panic(expected = "not two live entries")]
    fn get_disjoint_mut_rejects_the_same_entry_twice() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let a = app.create(1u32);
        let _ = app.get_disjoint_mut(a, a);
    }

    #[test]
    fn from_id_reads_like_the_minted_handle() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let minted = app.create(Counter(7));
        let rebuilt = Handle::<Counter>::from_id(minted.id());
        assert_eq!(rebuilt, minted);
        assert_eq!(app.get(rebuilt).0, 7);
    }

    #[test]
    #[should_panic(expected = "stale handle")]
    fn from_id_defers_the_stale_check_to_get() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let minted = app.create(Counter(0));
        let id = minted.id();
        app.destroy(minted);
        let rebuilt = Handle::<Counter>::from_id(id);
        app.get(rebuilt);
    }
}
