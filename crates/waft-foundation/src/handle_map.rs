//! The map behind [`App`](crate::App): every Flutter object is an entry here, named by a
//! [`Handle`]. Dart has no counterpart; see PORTING.md.
//!
//! An entry lives while something keeps it: a [`Retained`] (a [`Ref`] for a typed handle), or the
//! tree code, which owns elements, states, render objects and layers from
//! [`Retained::into_tree_node`] to [`HandleMap::release_tree_node`]. When the last keeper lets go,
//! the entry is freed at the next checkpoint, never at once: a bare [`Handle`] on the stack stays
//! good until then. Freeing drops the object, which drops the [`Ref`]s in its fields, so one sweep
//! can free a whole graph.
//!
//! The count is an `Rc` per entry, as [`Entity`](crate::Entity)'s is: the entry holds it weakly,
//! each [`Retained`] holds it strongly, and the last strong drop reports the id to the sweep.

use std::any::{Any, type_name};
use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
#[cfg(debug_assertions)]
use std::panic::Location;
use std::rc::{Rc, Weak};

use slotmap::SlotMap;

use crate::app::{App, Handle, HandleId};

struct HandleEntry {
    type_name: &'static str,
    state: Box<dyn Any>,
    /// The count every [`Retained`] of this entry shares. Weak here, so the entry does not keep
    /// itself.
    keep: Weak<Keep>,
    /// The tree's part of the count while the tree code owns this entry (an element, a state, a
    /// render object, a layer): taken by [`HandleMap::release_tree_node`].
    tree_owner: Option<Rc<Keep>>,
    /// Dart's `dispose` has run, whether or not something still keeps the object readable.
    disposed: bool,
    #[cfg(debug_assertions)]
    created_at: &'static Location<'static>,
}

/// One entry's count. Dropping the last one reports the entry to the sweep.
pub(crate) struct Keep {
    id: HandleId,
    dropped: DroppedEntries,
}

impl Drop for Keep {
    fn drop(&mut self) {
        self.dropped.push(self.id);
    }
}

/// The entries whose last keeper went since the last sweep, shared between the map and every
/// count it gave out, so a `Drop` can report itself without the map.
#[derive(Clone, Default)]
struct DroppedEntries(Rc<RefCell<Vec<HandleId>>>);

impl DroppedEntries {
    fn push(&self, id: HandleId) {
        self.0.borrow_mut().push(id);
    }

    fn take(&self) -> Vec<HandleId> {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}

/// A reference that keeps its object alive: Dart's ordinary reference.
///
/// `H` is the Copy handle it keeps: [`Handle<T>`] for [`Ref<T>`], an erased handle such as an
/// animation's for its alias, [`HandleId`] for [`AnyRef`]. It derefs to `H`, so every method of
/// the handle is callable on it. Clone shares the count; when the last `Retained` of an object is
/// dropped, the object is freed at the next checkpoint.
///
/// A field, a widget, a closure that is stored, anything kept past a checkpoint holds one of
/// these. Code that only uses an object during a call takes the bare handle.
pub struct Retained<H> {
    handle: H,
    keep: Rc<Keep>,
}

/// A [`Retained`] typed handle: what [`App::create`] returns and what a field Dart types as `T`
/// holds.
pub type Ref<T> = Retained<Handle<T>>;

/// A [`Retained`] untyped id, for a list that keeps objects of more than one type.
pub type AnyRef = Retained<HandleId>;

impl<H: Copy> Retained<H> {
    /// The handle this reference keeps alive.
    pub fn handle(&self) -> H {
        self.handle
    }

    /// The same object through another handle to it (an erased handle from a typed one), sharing
    /// this count.
    ///
    /// # Panics
    ///
    /// In debug builds, if `to` names another object.
    pub fn map<U: Copy + Into<HandleId>>(&self, to: impl FnOnce(H) -> U) -> Retained<U>
    where
        H: Into<HandleId>,
    {
        Retained {
            handle: same_object(self.handle, to(self.handle)),
            keep: Rc::clone(&self.keep),
        }
    }

    /// [`map`](Self::map), giving this reference's part of the count to the result.
    pub fn into_map<U: Copy + Into<HandleId>>(self, to: impl FnOnce(H) -> U) -> Retained<U>
    where
        H: Into<HandleId>,
    {
        Retained {
            handle: same_object(self.handle, to(self.handle)),
            keep: self.keep,
        }
    }

    /// This reference as the erased handle a field Dart types as the supertype holds (an
    /// `AnimationRef` from a `Ref<AnimationController>`), sharing the count: Dart's upcast. The
    /// erased handle's `From` impl names the conversion.
    pub fn erase<E: From<H> + Copy + Into<HandleId>>(&self) -> Retained<E>
    where
        H: Into<HandleId>,
    {
        self.map(E::from)
    }

    /// [`erase`](Self::erase), giving this reference's part of the count to the result.
    pub fn into_erased<E: From<H> + Copy + Into<HandleId>>(self) -> Retained<E>
    where
        H: Into<HandleId>,
    {
        self.into_map(E::from)
    }

    /// This reference as an untyped one, sharing the count.
    pub fn to_any(&self) -> AnyRef
    where
        H: Into<HandleId>,
    {
        self.map(Into::into)
    }

    /// Gives this reference's part of the count to the tree code, for a tree node (an element, a
    /// state, a render object, a layer), and answers the bare handle the tree keeps. The node then
    /// lives until [`App::release_tree_node`], or past it while another [`Retained`] keeps it.
    ///
    /// # Panics
    ///
    /// If the tree already owns the object.
    pub fn into_tree_node(self, app: &mut App) -> H
    where
        H: Into<HandleId>,
    {
        app.adopt_tree_node(self.handle.into(), self.keep);
        self.handle
    }
}

fn same_object<H: Copy + Into<HandleId>, U: Copy + Into<HandleId>>(from: H, to: U) -> U {
    debug_assert!(
        from.into() == to.into(),
        "a Retained can only be mapped to another handle of the same object"
    );
    to
}

impl<H: Copy> Clone for Retained<H> {
    fn clone(&self) -> Retained<H> {
        Retained {
            handle: self.handle,
            keep: Rc::clone(&self.keep),
        }
    }
}

impl<H> Deref for Retained<H> {
    type Target = H;

    fn deref(&self) -> &H {
        &self.handle
    }
}

/// Equal when they keep the same object, as Dart's `==` on an object reference.
impl<H: Copy + Into<HandleId>> PartialEq for Retained<H> {
    fn eq(&self, other: &Retained<H>) -> bool {
        self.handle.into() == other.handle.into()
    }
}

impl<H: Copy + Into<HandleId>> Eq for Retained<H> {}

impl<H: Copy + Into<HandleId>> Hash for Retained<H> {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.handle.into().hash(state)
    }
}

impl<H: fmt::Debug> fmt::Debug for Retained<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Retained({:?})", self.handle)
    }
}

/// A set or map keyed by `Retained` is looked up by the bare handle: both hash and compare by
/// the object's id.
impl<H> std::borrow::Borrow<H> for Retained<H> {
    fn borrow(&self) -> &H {
        &self.handle
    }
}

impl<H: Copy + Into<HandleId>> From<&Retained<H>> for HandleId {
    fn from(retained: &Retained<H>) -> HandleId {
        retained.handle.into()
    }
}

/// The bare handle of an optional reference, for a getter that answers what a field keeps
/// without handing out a second keeper: `app.get(self).controller.handle()`.
pub trait OptionRetained<H> {
    /// The kept handle, if any.
    fn handle(&self) -> Option<H>;
}

impl<H: Copy> OptionRetained<H> for Option<Retained<H>> {
    fn handle(&self) -> Option<H> {
        self.as_ref().map(Retained::handle)
    }
}

/// What a debug build remembers of a freed entry, for the stale-read message.
#[cfg(debug_assertions)]
struct FreedEntry {
    type_name: &'static str,
    created_at: &'static Location<'static>,
    checkpoint: u64,
}

/// How many freed entries a debug build remembers for the stale-read message.
#[cfg(debug_assertions)]
const FREED_ENTRIES_REMEMBERED: usize = 1 << 16;

/// The entries, the counts dropped since the last sweep, and the checkpoint count.
pub(crate) struct HandleMap {
    entries: SlotMap<HandleId, HandleEntry>,
    dropped: DroppedEntries,
    /// The checkpoints begun so far; a free is dated by the one it ran in.
    checkpoint: u64,
    #[cfg(debug_assertions)]
    freed: std::collections::HashMap<HandleId, FreedEntry>,
    #[cfg(debug_assertions)]
    freed_order: std::collections::VecDeque<HandleId>,
}

impl HandleMap {
    pub(crate) fn new() -> HandleMap {
        HandleMap {
            entries: SlotMap::with_key(),
            dropped: DroppedEntries::default(),
            checkpoint: 0,
            #[cfg(debug_assertions)]
            freed: std::collections::HashMap::new(),
            #[cfg(debug_assertions)]
            freed_order: std::collections::VecDeque::new(),
        }
    }

    #[track_caller]
    pub(crate) fn create<T: 'static>(&mut self, state: T) -> Ref<T> {
        #[cfg(debug_assertions)]
        let created_at = Location::caller();
        let dropped = &self.dropped;
        let mut keep = None;
        let id = self.entries.insert_with_key(|id| {
            let strong = Rc::new(Keep {
                id,
                dropped: dropped.clone(),
            });
            let entry = HandleEntry {
                type_name: type_name::<T>(),
                state: Box::new(state),
                keep: Rc::downgrade(&strong),
                tree_owner: None,
                disposed: false,
                #[cfg(debug_assertions)]
                created_at,
            };
            keep = Some(strong);
            entry
        });
        Retained {
            handle: Handle::new(id),
            keep: keep.expect("the entry was inserted"),
        }
    }

    /// A [`Ref`] to the entry while something still keeps it; `None` once it is freed or its
    /// last keeper has let go, even before the sweep.
    ///
    /// # Panics
    ///
    /// If the entry holds another type.
    pub(crate) fn upgrade<T: 'static>(&self, handle: Handle<T>) -> Option<Ref<T>> {
        let entry = self.entries.get(handle.id())?;
        assert!(
            entry.state.is::<T>(),
            "handle {:?} holds `{}`, not `{}`",
            handle.id(),
            entry.type_name,
            type_name::<T>()
        );
        Some(Retained {
            handle,
            keep: entry.keep.upgrade()?,
        })
    }

    /// [`upgrade`](Self::upgrade) for an untyped id.
    pub(crate) fn upgrade_id(&self, id: HandleId) -> Option<AnyRef> {
        Some(Retained {
            handle: id,
            keep: self.entries.get(id)?.keep.upgrade()?,
        })
    }

    pub(crate) fn adopt_tree_node(&mut self, id: HandleId, keep: Rc<Keep>) {
        let entry = self.resolve_mut(id);
        assert!(
            entry.tree_owner.is_none(),
            "a `{}` is already a tree node",
            entry.type_name
        );
        entry.tree_owner = Some(keep);
    }

    /// The tree code is done with a node: Dart's lifecycle has ended it (an element unmounted, a
    /// state or render object or layer disposed). Marks it disposed and gives up the tree's part
    /// of the count, so it is freed at the next checkpoint unless a [`Retained`] still keeps it.
    ///
    /// # Panics
    ///
    /// If the entry is not a tree node, or the tree has released it already.
    pub(crate) fn release_tree_node(&mut self, id: HandleId) {
        let entry = self.resolve_mut(id);
        entry.disposed = true;
        let tree_owner = entry.tree_owner.take().unwrap_or_else(|| {
            panic!(
                "a `{}` that the tree does not own was released as a tree node",
                entry.type_name
            )
        });
        drop(tree_owner);
    }

    /// Whether the entry exists: kept, or let go and waiting for the next checkpoint.
    pub(crate) fn contains(&self, id: HandleId) -> bool {
        self.entries.contains_key(id)
    }

    /// Dart's `dispose` ran on the object: a fact for tooling, which keeps it readable.
    pub(crate) fn mark_disposed(&mut self, id: HandleId) {
        self.resolve_mut(id).disposed = true;
    }

    /// Whether Dart's `dispose` has run on the object. `false` for a freed one.
    pub(crate) fn is_disposed(&self, id: HandleId) -> bool {
        self.entries.get(id).is_some_and(|entry| entry.disposed)
    }

    /// A checkpoint begins: the frees from here on are dated by it.
    pub(crate) fn begin_checkpoint(&mut self) {
        self.checkpoint += 1;
    }

    /// Frees every entry whose last keeper went since the last sweep, and the entries those
    /// frees let go in turn.
    pub(crate) fn free_dropped(&mut self) {
        loop {
            let dropped = self.dropped.take();
            if dropped.is_empty() {
                return;
            }
            for id in dropped {
                let Some(entry) = self.entries.remove(id) else {
                    continue;
                };
                debug_assert!(
                    entry.keep.strong_count() == 0,
                    "a `{}` was freed while kept",
                    entry.type_name
                );
                self.remember_freed(id, &entry);
                // Dropping the object drops the references in its fields, which report
                // themselves for the next round of this loop.
                drop(entry);
            }
        }
    }

    #[cfg(debug_assertions)]
    fn remember_freed(&mut self, id: HandleId, entry: &HandleEntry) {
        if self.freed_order.len() == FREED_ENTRIES_REMEMBERED
            && let Some(oldest) = self.freed_order.pop_front()
        {
            self.freed.remove(&oldest);
        }
        self.freed_order.push_back(id);
        self.freed.insert(
            id,
            FreedEntry {
                type_name: entry.type_name,
                created_at: entry.created_at,
                checkpoint: self.checkpoint,
            },
        );
    }

    #[cfg(not(debug_assertions))]
    fn remember_freed(&mut self, _id: HandleId, _entry: &HandleEntry) {}

    /// The type and creation site of a live entry, for a debug message.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_describe(&self, id: HandleId) -> String {
        if let Some(entry) = self.entries.get(id) {
            return format!("a `{}` created at {}", entry.type_name, entry.created_at);
        }
        self.stale(id)
    }

    /// The number of entries, kept or waiting for the next checkpoint.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_entry_count(&self) -> usize {
        self.entries.len()
    }

    /// The number of entries of each type, kept or waiting for the next checkpoint.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_entry_counts_by_type(
        &self,
    ) -> std::collections::BTreeMap<&'static str, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for entry in self.entries.values() {
            *counts.entry(entry.type_name).or_insert(0) += 1;
        }
        counts
    }

    /// Narrow an id. `None` if freed or the wrong type; [`get`](HandleMap::get) panics instead.
    pub(crate) fn handle<T: 'static>(&self, id: HandleId) -> Option<Handle<T>> {
        if !self.entries.get(id)?.state.is::<T>() {
            return None;
        }
        Some(Handle::new(id))
    }

    /// # Panics
    ///
    /// If the object is freed.
    pub(crate) fn get<T: 'static>(&self, handle: Handle<T>) -> &T {
        let entry = self.resolve(handle.id());
        entry.state.downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "handle {:?} holds `{}`, not `{}`",
                handle.id(),
                entry.type_name,
                type_name::<T>()
            )
        })
    }

    pub(crate) fn get_mut<T: 'static>(&mut self, handle: Handle<T>) -> &mut T {
        let id = handle.id();
        let entry = self.resolve_mut(id);
        downcast_entry_mut(entry, id)
    }

    /// Two live entries at once.
    ///
    /// # Panics
    ///
    /// If either object is freed, or both handles name the same entry.
    pub(crate) fn get_disjoint_mut<A: 'static, B: 'static>(
        &mut self,
        a: Handle<A>,
        b: Handle<B>,
    ) -> (&mut A, &mut B) {
        if a.id() == b.id() {
            panic!(
                "handles {:?} and {:?} are not two live entries",
                a.id(),
                b.id()
            );
        }
        if !self.entries.contains_key(a.id()) {
            panic!("{}", self.stale(a.id()));
        }
        if !self.entries.contains_key(b.id()) {
            panic!("{}", self.stale(b.id()));
        }
        let [entry_a, entry_b] = self
            .entries
            .get_disjoint_mut([a.id(), b.id()])
            .expect("two live entries");
        (
            downcast_entry_mut(entry_a, a.id()),
            downcast_entry_mut(entry_b, b.id()),
        )
    }

    fn resolve(&self, id: HandleId) -> &HandleEntry {
        match self.entries.get(id) {
            Some(entry) => entry,
            None => panic!("{}", self.stale(id)),
        }
    }

    fn resolve_mut(&mut self, id: HandleId) -> &mut HandleEntry {
        if !self.entries.contains_key(id) {
            panic!("{}", self.stale(id));
        }
        self.entries.get_mut(id).expect("checked above")
    }

    /// The stale-read message: the object's type, where it was made and when it was freed, while
    /// a debug build remembers them.
    fn stale(&self, id: HandleId) -> String {
        #[cfg(debug_assertions)]
        if let Some(freed) = self.freed.get(&id) {
            return format!(
                "stale handle: {id:?} is a `{}` created at {}, freed at checkpoint {}",
                freed.type_name, freed.created_at, freed.checkpoint
            );
        }
        format!("stale handle: {id:?} was freed")
    }
}

fn downcast_entry_mut<T: 'static>(entry: &mut HandleEntry, id: HandleId) -> &mut T {
    let type_name_in_entry = entry.type_name;
    entry.state.downcast_mut::<T>().unwrap_or_else(|| {
        panic!(
            "handle {id:?} holds `{type_name_in_entry}`, not `{}`",
            type_name::<T>()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Holder(#[allow(dead_code, reason = "kept for its drop")] Ref<u32>);

    #[test]
    fn an_object_lives_while_a_ref_keeps_it_and_is_freed_by_the_sweep() {
        let mut handles = HandleMap::new();
        let first = handles.create(7u32);
        let second = first.clone();
        let handle = *first;
        drop(first);
        handles.free_dropped();
        assert_eq!(*handles.get(handle), 7, "the clone keeps it");
        drop(second);
        assert!(
            handles.contains(handle.id()),
            "the drop alone does not free it"
        );
        assert!(
            handles.upgrade(handle).is_none(),
            "but it cannot be upgraded"
        );
        handles.free_dropped();
        assert!(!handles.contains(handle.id()), "the sweep did");
    }

    #[test]
    fn freeing_an_object_frees_what_its_fields_kept() {
        let mut handles = HandleMap::new();
        let inner = handles.create(1u32);
        let inner_handle = *inner;
        let outer = handles.create(Holder(inner));
        drop(outer);
        handles.free_dropped();
        assert!(
            !handles.contains(inner_handle.id()),
            "one sweep frees the chain"
        );
    }

    #[test]
    fn upgrade_answers_a_ref_while_the_object_is_kept() {
        let mut handles = HandleMap::new();
        let first = handles.create(3u8);
        let upgraded = handles.upgrade(*first).expect("kept");
        drop(first);
        handles.free_dropped();
        assert_eq!(*handles.get(*upgraded), 3, "the upgraded ref keeps it");
    }

    #[test]
    fn a_tree_node_lives_until_the_tree_releases_it() {
        let cell = crate::AppCell::new();
        let mut app = cell.borrow_mut();
        let node = app.create(5u16).into_tree_node(&mut app);
        drop(app);
        cell.checkpoint();
        assert!(cell.borrow().contains(node), "the tree keeps it");
        cell.borrow_mut().release_tree_node(node);
        assert!(cell.borrow().is_disposed(node));
        cell.checkpoint();
        assert!(
            !cell.borrow().contains(node),
            "freed at the checkpoint after the release"
        );
    }

    #[test]
    fn a_released_tree_node_still_kept_by_a_ref_stays_readable() {
        let cell = crate::AppCell::new();
        let mut app = cell.borrow_mut();
        let node = app.create(5u16).into_tree_node(&mut app);
        let kept = node.upgrade(&app).expect("the tree keeps it");
        app.release_tree_node(node);
        drop(app);
        cell.checkpoint();
        assert_eq!(*cell.borrow().get(*kept), 5);
    }

    #[test]
    #[should_panic(expected = "the tree does not own")]
    fn releasing_a_node_twice_panics() {
        let cell = crate::AppCell::new();
        let mut app = cell.borrow_mut();
        let node = app.create(5u16).into_tree_node(&mut app);
        let _kept = node.upgrade(&app);
        app.release_tree_node(node);
        app.release_tree_node(node);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn a_stale_read_names_the_type_the_creation_site_and_the_checkpoint() {
        let cell = crate::AppCell::new();
        let handle = *cell.borrow_mut().create(9u64);
        cell.checkpoint();
        let message = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = cell.borrow().get(handle);
        }))
        .expect_err("a stale read panics");
        let message = message
            .downcast_ref::<String>()
            .expect("a formatted message");
        assert!(message.contains("`u64`"), "{message}");
        assert!(message.contains("handle_map.rs"), "{message}");
        assert!(message.contains("freed at checkpoint 1"), "{message}");
    }
}
