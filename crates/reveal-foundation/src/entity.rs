//! App-level stores the user writes. Not a Flutter type and not how the framework is implemented.
//!
//! [`Handle`](crate::Handle) is the Flutter object: Copy, point access, no lease. [`Entity`] is
//! Clone and refcounted; [`update`](Entity::update) leases `T` out of the map so the store can
//! [`notify`](Context::notify) and [`emit`](Context::emit) without naming itself. Re-entering the
//! same entity panics. Other entities and every Handle stay visible.

use std::any::{Any, TypeId, type_name};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};

use slotmap::{SlotMap, new_key_type};

use crate::app::App;

new_key_type! {
    /// Identity of an [`Entity`]. Stale after the last strong handle drops and effects flush.
    pub struct EntityId;
}

/// Entities a tracking frame read or updated. [`App::entity_track`] returns this.
#[derive(Clone, Debug, Default)]
pub struct TrackedSet {
    ids: HashSet<EntityId>,
}

impl TrackedSet {
    pub fn contains(&self, id: EntityId) -> bool {
        self.ids.contains(&id)
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.ids.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }
}

/// Marker: `T` can [`Context::emit`] events of type `E`.
pub trait EventEmitter<E: 'static>: 'static {}

/// A handle to an [`observe`](App::observe) or [`subscribe`](App::subscribe). Drop cancels it.
/// [`detach`](Self::detach) keeps the callback until the emitter is gone.
#[must_use]
pub struct Subscription {
    cancel: Option<Rc<Cell<bool>>>,
}

impl Subscription {
    /// Keep the callback after this handle is dropped.
    pub fn detach(mut self) {
        self.cancel.take();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.set(true);
        }
    }
}

impl Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Subscription")
    }
}

/// Strong, refcounted handle to a user store owned by [`App`].
pub struct Entity<T> {
    id: EntityId,
    keep: Rc<EntityKeep>,
    entity_type: PhantomData<fn(T) -> T>,
}

/// Weak handle. [`upgrade`](Self::upgrade) fails as soon as the last [`Entity`] drops.
pub struct WeakEntity<T> {
    id: EntityId,
    keep: Weak<EntityKeep>,
    entity_type: PhantomData<fn(T) -> T>,
}

struct EntityKeep {
    id: EntityId,
    dropped: DroppedEntities,
}

impl Drop for EntityKeep {
    fn drop(&mut self) {
        self.dropped.push(self.id);
    }
}

#[derive(Clone, Default)]
struct DroppedEntities(Rc<RefCell<Vec<EntityId>>>);

impl DroppedEntities {
    fn push(&self, id: EntityId) {
        self.0.borrow_mut().push(id);
    }

    fn take(&self) -> Vec<EntityId> {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}

struct Slot {
    type_name: &'static str,
    type_id: TypeId,
    value: Option<Box<dyn Any>>,
    keep: Weak<EntityKeep>,
}

enum Effect {
    Notify {
        emitter: EntityId,
    },
    Emit {
        emitter: EntityId,
        event_type: TypeId,
        event: Box<dyn Any>,
    },
}

struct Observer {
    active: bool,
    cancelled: Rc<Cell<bool>>,
    handler: Box<dyn FnMut(&mut App) -> bool>,
}

type EventHandler = Box<dyn FnMut(&mut App, &dyn Any) -> bool>;

struct EventListener {
    active: bool,
    cancelled: Rc<Cell<bool>>,
    event_type: TypeId,
    handler: EventHandler,
}

pub(crate) struct EntityMap {
    slots: SlotMap<EntityId, Slot>,
    dropped: DroppedEntities,
    tracking: Rc<RefCell<Vec<HashSet<EntityId>>>>,
    update_depth: u32,
    flushing: bool,
    pending: VecDeque<Effect>,
    pending_notifications: HashSet<EntityId>,
    observers: HashMap<EntityId, Vec<Observer>>,
    event_listeners: HashMap<EntityId, Vec<EventListener>>,
}

impl EntityMap {
    pub(crate) fn new() -> EntityMap {
        EntityMap {
            slots: SlotMap::with_key(),
            dropped: DroppedEntities::default(),
            tracking: Rc::new(RefCell::new(Vec::new())),
            update_depth: 0,
            flushing: false,
            pending: VecDeque::new(),
            pending_notifications: HashSet::new(),
            observers: HashMap::new(),
            event_listeners: HashMap::new(),
        }
    }
}

impl<T> Entity<T> {
    pub fn entity_id(&self) -> EntityId {
        self.id
    }

    pub fn downgrade(&self) -> WeakEntity<T> {
        WeakEntity {
            id: self.id,
            keep: Rc::downgrade(&self.keep),
            entity_type: PhantomData,
        }
    }

    /// Records this id when a tracking frame is open.
    pub fn read<'a>(&self, app: &'a App) -> &'a T
    where
        T: 'static,
    {
        app.record_entity_access(self.id);
        app.entity_ref(self.id)
    }

    /// Leases `T` for the closure. Re-entering this entity panics.
    pub fn update<R>(&self, app: &mut App, f: impl FnOnce(&mut T, &mut Context<T>) -> R) -> R
    where
        T: 'static,
    {
        app.record_entity_access(self.id);
        app.update_entity(self, f)
    }
}

impl<T> Clone for Entity<T> {
    fn clone(&self) -> Entity<T> {
        Entity {
            id: self.id,
            keep: Rc::clone(&self.keep),
            entity_type: PhantomData,
        }
    }
}

impl<T> PartialEq for Entity<T> {
    fn eq(&self, other: &Entity<T>) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Entity<T> {}

impl<T> Hash for Entity<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> Debug for Entity<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity<{}>({:?})", type_name::<T>(), self.id)
    }
}

impl<T> WeakEntity<T> {
    pub fn entity_id(&self) -> EntityId {
        self.id
    }

    pub fn upgrade(&self) -> Option<Entity<T>> {
        self.keep.upgrade().map(|keep| Entity {
            id: self.id,
            keep,
            entity_type: PhantomData,
        })
    }
}

impl<T> Clone for WeakEntity<T> {
    fn clone(&self) -> WeakEntity<T> {
        WeakEntity {
            id: self.id,
            keep: Weak::clone(&self.keep),
            entity_type: PhantomData,
        }
    }
}

impl<T> Debug for WeakEntity<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WeakEntity<{}>({:?})", type_name::<T>(), self.id)
    }
}

/// `&mut App` scoped to one entity so the store can notify and emit while `T` is leased.
pub struct Context<'a, T> {
    app: &'a mut App,
    entity: WeakEntity<T>,
}

impl<T> Context<'_, T> {
    fn new<'a>(app: &'a mut App, entity: WeakEntity<T>) -> Context<'a, T> {
        Context { app, entity }
    }

    pub fn entity(&self) -> Entity<T> {
        self.entity
            .upgrade()
            .expect("the entity is leased, so a strong handle exists")
    }

    pub fn weak_entity(&self) -> WeakEntity<T> {
        self.entity.clone()
    }

    /// Untyped dirty: observers run, and elements that read this entity last build rebuild.
    pub fn notify(&mut self) {
        let id = self.entity.id;
        self.app.queue_entity_notify(id);
    }

    pub fn emit<E>(&mut self, event: E)
    where
        T: EventEmitter<E>,
        E: 'static,
    {
        let id = self.entity.id;
        self.app.queue_entity_emit(id, event);
    }

    pub fn observe<W: 'static>(
        &mut self,
        entity: &Entity<W>,
        mut on_notify: impl FnMut(&mut T, Entity<W>, &mut Context<T>) + 'static,
    ) -> Subscription
    where
        T: 'static,
    {
        let this = self.weak_entity();
        let watched = entity.clone();
        self.app.observe_entity_id(entity.entity_id(), move |app| {
            let Some(this) = this.upgrade() else {
                return false;
            };
            this.update(app, |this, cx| on_notify(this, watched.clone(), cx));
            true
        })
    }

    pub fn subscribe<W, E>(
        &mut self,
        entity: &Entity<W>,
        mut on_event: impl FnMut(&mut T, Entity<W>, &E, &mut Context<T>) + 'static,
    ) -> Subscription
    where
        T: 'static,
        W: EventEmitter<E> + 'static,
        E: 'static,
    {
        let this = self.weak_entity();
        let watched = entity.clone();
        self.app.subscribe_entity_id(
            entity.entity_id(),
            TypeId::of::<E>(),
            move |app, event| {
                let Some(this) = this.upgrade() else {
                    return false;
                };
                let event = event
                    .downcast_ref::<E>()
                    .expect("subscriber event type matches the emitter");
                this.update(app, |this, cx| {
                    on_event(this, watched.clone(), event, cx)
                });
                true
            },
        )
    }
}

impl<T> Deref for Context<'_, T> {
    type Target = App;

    fn deref(&self) -> &App {
        self.app
    }
}

impl<T> DerefMut for Context<'_, T> {
    fn deref_mut(&mut self) -> &mut App {
        self.app
    }
}

struct TrackFrame {
    tracking: Rc<RefCell<Vec<HashSet<EntityId>>>>,
    taken: bool,
}

impl TrackFrame {
    fn finish(mut self) -> HashSet<EntityId> {
        self.taken = true;
        self.tracking
            .borrow_mut()
            .pop()
            .expect("entity_track frame")
    }
}

impl Drop for TrackFrame {
    fn drop(&mut self) {
        if !self.taken {
            self.tracking.borrow_mut().pop();
        }
    }
}

impl App {
    /// Opens a tracking frame. [`Entity::read`] and [`Entity::update`] record into it.
    pub fn entity_track<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> (R, TrackedSet) {
        self.entities.tracking.borrow_mut().push(HashSet::new());
        let frame = TrackFrame {
            tracking: Rc::clone(&self.entities.tracking),
            taken: false,
        };
        let result = f(self);
        (result, TrackedSet { ids: frame.finish() })
    }

    pub fn new_entity<T: 'static>(&mut self, build: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        self.begin_entity_update();
        let entity = self.reserve_entity::<T>();
        let value = {
            let mut cx = Context::new(self, entity.downgrade());
            build(&mut cx)
        };
        self.insert_entity(&entity, value);
        self.record_entity_access(entity.id);
        self.end_entity_update();
        entity
    }

    pub fn observe<T: 'static>(
        &mut self,
        entity: &Entity<T>,
        mut on_notify: impl FnMut(&mut App) + 'static,
    ) -> Subscription {
        self.observe_entity_id(entity.id, move |app| {
            on_notify(app);
            true
        })
    }

    pub fn subscribe<T, E>(
        &mut self,
        entity: &Entity<T>,
        mut on_event: impl FnMut(&mut App, &E) + 'static,
    ) -> Subscription
    where
        T: EventEmitter<E> + 'static,
        E: 'static,
    {
        self.subscribe_entity_id(entity.id, TypeId::of::<E>(), move |app, event| {
            let event = event
                .downcast_ref::<E>()
                .expect("subscriber event type matches the emitter");
            on_event(app, event);
            true
        })
    }

    /// Runs queued notify/emit and activates subscribers registered since the last flush.
    pub fn flush_entity_effects(&mut self) {
        if self.entities.flushing {
            return;
        }
        self.entities.flushing = true;
        loop {
            self.release_dropped_entities();
            if let Some(effect) = self.entities.pending.pop_front() {
                match effect {
                    Effect::Notify { emitter } => self.apply_entity_notify(emitter),
                    Effect::Emit {
                        emitter,
                        event_type,
                        event,
                    } => self.apply_entity_emit(emitter, event_type, &*event),
                }
                continue;
            }
            self.activate_entity_subscribers();
            if self.entities.pending.is_empty() {
                break;
            }
        }
        self.entities.flushing = false;
    }

    pub(crate) fn record_entity_access(&self, id: EntityId) {
        if let Some(frame) = self.entities.tracking.borrow_mut().last_mut() {
            frame.insert(id);
        }
    }

    pub fn observe_entity_id(
        &mut self,
        id: EntityId,
        handler: impl FnMut(&mut App) -> bool + 'static,
    ) -> Subscription {
        let cancelled = Rc::new(Cell::new(false));
        self.entities.observers.entry(id).or_default().push(Observer {
            active: false,
            cancelled: Rc::clone(&cancelled),
            handler: Box::new(handler),
        });
        if self.entities.update_depth == 0 && !self.entities.flushing {
            self.flush_entity_effects();
        }
        Subscription {
            cancel: Some(cancelled),
        }
    }

    fn subscribe_entity_id(
        &mut self,
        id: EntityId,
        event_type: TypeId,
        handler: impl FnMut(&mut App, &dyn Any) -> bool + 'static,
    ) -> Subscription {
        let cancelled = Rc::new(Cell::new(false));
        self.entities
            .event_listeners
            .entry(id)
            .or_default()
            .push(EventListener {
                active: false,
                cancelled: Rc::clone(&cancelled),
                event_type,
                handler: Box::new(handler),
            });
        if self.entities.update_depth == 0 && !self.entities.flushing {
            self.flush_entity_effects();
        }
        Subscription {
            cancel: Some(cancelled),
        }
    }

    fn begin_entity_update(&mut self) {
        self.entities.update_depth += 1;
    }

    fn end_entity_update(&mut self) {
        self.entities.update_depth = self.entities.update_depth.saturating_sub(1);
        if self.entities.update_depth == 0 {
            self.flush_entity_effects();
        }
    }

    fn queue_entity_notify(&mut self, id: EntityId) {
        if self.entities.pending_notifications.insert(id) {
            self.entities
                .pending
                .push_back(Effect::Notify { emitter: id });
        }
        if self.entities.update_depth == 0 {
            self.flush_entity_effects();
        }
    }

    fn queue_entity_emit<E: 'static>(&mut self, id: EntityId, event: E) {
        self.entities.pending.push_back(Effect::Emit {
            emitter: id,
            event_type: TypeId::of::<E>(),
            event: Box::new(event),
        });
        if self.entities.update_depth == 0 {
            self.flush_entity_effects();
        }
    }

    fn activate_entity_subscribers(&mut self) {
        for observers in self.entities.observers.values_mut() {
            for observer in observers.iter_mut() {
                if !observer.cancelled.get() {
                    observer.active = true;
                }
            }
        }
        for listeners in self.entities.event_listeners.values_mut() {
            for listener in listeners.iter_mut() {
                if !listener.cancelled.get() {
                    listener.active = true;
                }
            }
        }
    }

    fn apply_entity_notify(&mut self, emitter: EntityId) {
        self.entities.pending_notifications.remove(&emitter);
        let mut observers = self
            .entities
            .observers
            .remove(&emitter)
            .unwrap_or_default();
        let mut i = 0;
        while i < observers.len() {
            let run = !observers[i].cancelled.get() && observers[i].active;
            if run && !(observers[i].handler)(self) {
                observers[i].cancelled.set(true);
            }
            if let Some(mut extra) = self.entities.observers.remove(&emitter) {
                observers.append(&mut extra);
            }
            i += 1;
        }
        observers.retain(|observer| !observer.cancelled.get());
        if !observers.is_empty() {
            self.entities.observers.insert(emitter, observers);
        }
    }

    fn apply_entity_emit(&mut self, emitter: EntityId, event_type: TypeId, event: &dyn Any) {
        let mut listeners = self
            .entities
            .event_listeners
            .remove(&emitter)
            .unwrap_or_default();
        let mut i = 0;
        while i < listeners.len() {
            let run = !listeners[i].cancelled.get()
                && listeners[i].active
                && listeners[i].event_type == event_type;
            if run && !(listeners[i].handler)(self, event) {
                listeners[i].cancelled.set(true);
            }
            if let Some(mut extra) = self.entities.event_listeners.remove(&emitter) {
                listeners.append(&mut extra);
            }
            i += 1;
        }
        listeners.retain(|listener| !listener.cancelled.get());
        if !listeners.is_empty() {
            self.entities.event_listeners.insert(emitter, listeners);
        }
    }

    fn release_dropped_entities(&mut self) {
        for id in self.entities.dropped.take() {
            if self
                .entities
                .slots
                .get(id)
                .is_some_and(|slot| slot.keep.upgrade().is_some())
            {
                continue;
            }
            self.entities.slots.remove(id);
            self.entities.observers.remove(&id);
            self.entities.event_listeners.remove(&id);
            self.entities.pending_notifications.remove(&id);
        }
    }

    fn reserve_entity<T: 'static>(&mut self) -> Entity<T> {
        let dropped = self.entities.dropped.clone();
        let id = self.entities.slots.insert(Slot {
            type_name: type_name::<T>(),
            type_id: TypeId::of::<T>(),
            value: None,
            keep: Weak::new(),
        });
        let keep = Rc::new(EntityKeep {
            id,
            dropped,
        });
        self.entities.slots[id].keep = Rc::downgrade(&keep);
        Entity {
            id,
            keep,
            entity_type: PhantomData,
        }
    }

    fn insert_entity<T: 'static>(&mut self, entity: &Entity<T>, value: T) {
        let slot = self
            .entities
            .slots
            .get_mut(entity.id)
            .expect("reserved entity");
        assert!(slot.value.is_none(), "insert into an occupied entity slot");
        slot.value = Some(Box::new(value));
    }

    fn entity_ref<T: 'static>(&self, id: EntityId) -> &T {
        let slot = self.entities.slots.get(id).unwrap_or_else(|| {
            panic!("stale entity {}", type_name::<T>())
        });
        assert_eq!(
            slot.type_id,
            TypeId::of::<T>(),
            "entity type mismatch: stored {}, asked {}",
            slot.type_name,
            type_name::<T>()
        );
        let value = slot.value.as_ref().unwrap_or_else(|| {
            panic!(
                "cannot read {} while it is already being updated",
                slot.type_name
            )
        });
        value.downcast_ref::<T>().expect("entity type matches slot")
    }

    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Entity<T>,
        f: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> R {
        self.begin_entity_update();
        let mut value = self.take_entity::<T>(entity.id);
        let result = {
            let mut cx = Context::new(self, entity.downgrade());
            f(&mut value, &mut cx)
        };
        self.put_entity(entity.id, value);
        self.end_entity_update();
        result
    }

    fn take_entity<T: 'static>(&mut self, id: EntityId) -> T {
        let slot = self.entities.slots.get_mut(id).unwrap_or_else(|| {
            panic!("stale entity {}", type_name::<T>())
        });
        assert_eq!(slot.type_id, TypeId::of::<T>());
        let boxed = slot.value.take().unwrap_or_else(|| {
            panic!(
                "cannot update {} while it is already being updated",
                slot.type_name
            )
        });
        *boxed
            .downcast::<T>()
            .unwrap_or_else(|_| panic!("entity type matches slot"))
    }

    fn put_entity<T: 'static>(&mut self, id: EntityId, value: T) {
        let slot = self
            .entities
            .slots
            .get_mut(id)
            .expect("entity slot exists for a live lease");
        slot.value = Some(Box::new(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppCell;
    use std::rc::Rc;

    struct Counter {
        count: i32,
    }

    struct Doubler {
        count: i32,
    }

    struct Changed {
        by: i32,
    }

    impl EventEmitter<Changed> for Counter {}

    #[test]
    fn read_and_update_round_trip() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.new_entity(|_cx| Counter { count: 1 });
        assert_eq!(counter.read(&app).count, 1);
        counter.update(&mut app, |counter, cx| {
            counter.count += 1;
            cx.notify();
        });
        assert_eq!(counter.read(&app).count, 2);
    }

    #[test]
    #[should_panic(expected = "already being updated")]
    fn reentering_the_same_entity_panics() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.new_entity(|_cx| Counter { count: 0 });
        counter.update(&mut app, |_counter, cx| {
            let this = cx.entity();
            this.read(cx);
        });
    }

    #[test]
    fn entity_track_records_reads() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.new_entity(|_cx| Counter { count: 3 });
        let other = app.new_entity(|_cx| Counter { count: 0 });
        let (n, tracked) = app.entity_track(|app| counter.read(app).count);
        assert_eq!(n, 3);
        assert!(tracked.contains(counter.entity_id()));
        assert!(!tracked.contains(other.entity_id()));
    }

    #[test]
    fn observe_runs_after_notify() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = app.new_entity(|_cx| Counter { count: 1 });
        let second = app.new_entity(|cx| {
            cx.observe(&first, |second: &mut Doubler, first, cx| {
                second.count = first.read(cx).count * 2;
            })
            .detach();
            Doubler { count: 0 }
        });
        first.update(&mut app, |counter, cx| {
            counter.count = 4;
            cx.notify();
        });
        assert_eq!(second.read(&app).count, 8);
    }

    #[test]
    fn a_subscriber_registered_during_notify_does_not_see_that_notify() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(Cell::new(0));
        let first = app.new_entity(|_cx| Counter { count: 0 });
        let seen_for_late = Rc::clone(&seen);
        first.update(&mut app, |counter, cx| {
            counter.count = 1;
            let late = cx.entity();
            cx.observe(&late, {
                let seen = Rc::clone(&seen_for_late);
                move |_, _, _| seen.set(seen.get() + 1)
            })
            .detach();
            cx.notify();
        });
        assert_eq!(seen.get(), 0);
        first.update(&mut app, |_counter, cx| cx.notify());
        assert_eq!(seen.get(), 1);
    }

    #[test]
    fn emit_reaches_a_typed_subscriber() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = app.new_entity(|_cx| Counter { count: 0 });
        let second = app.new_entity(|cx| {
            cx.subscribe(&first, |second: &mut Doubler, _first, event: &Changed, _cx| {
                second.count += event.by;
            })
            .detach();
            Doubler { count: 0 }
        });
        first.update(&mut app, |_counter, cx| {
            cx.emit(Changed { by: 5 });
        });
        assert_eq!(second.read(&app).count, 5);
    }

    #[test]
    fn dropping_the_last_handle_reaps_the_entity() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter = app.new_entity(|_cx| Counter { count: 1 });
        let weak = counter.downgrade();
        drop(counter);
        app.flush_entity_effects();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn dropping_a_subscription_stops_callbacks() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(Cell::new(0));
        let counter = app.new_entity(|_cx| Counter { count: 0 });
        let sub = {
            let seen = Rc::clone(&seen);
            app.observe(&counter, move |_app| seen.set(seen.get() + 1))
        };
        counter.update(&mut app, |_c, cx| cx.notify());
        assert_eq!(seen.get(), 1);
        drop(sub);
        counter.update(&mut app, |_c, cx| cx.notify());
        assert_eq!(seen.get(), 1);
    }
}
