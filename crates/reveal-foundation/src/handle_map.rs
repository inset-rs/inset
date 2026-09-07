//! The map behind [`App`](crate::App): every Flutter object is an entry here, named by a
//! [`Handle`]. Dart has no counterpart; see PORTING.md.
//!
//! An entry is kept by its owner from [`HandleMap::create`] until [`HandleMap::destroy`], and by
//! each [`RetainedHandle`] a cross-turn holder takes with [`HandleMap::retain`]. The last of those
//! to go removes the entry. `destroy` is Dart's `dispose`: after it the object is out of every tree and
//! stays readable only while a retained handle keeps it, the way a Dart object stays alive while
//! something still references it.

use std::any::{Any, type_name};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use slotmap::SlotMap;

use crate::app::{Handle, HandleId};

struct HandleEntry {
    type_name: &'static str,
    state: Box<dyn Any>,
    /// Who keeps the object: the owner, from `create`, plus one per [`RetainedHandle`]. The entry
    /// is removed when this reaches zero.
    retained: u32,
    /// The owner called `destroy`: Dart's `dispose` has run, whether or not a retained handle
    /// still keeps the object readable.
    disposed: bool,
}

/// Keeps an object past its owner's `destroy`, from [`HandleMap::retain`]. Give it back with
/// [`HandleMap::release`] to remove the entry at once; one that is merely dropped is released at
/// the next checkpoint, the way gpui sweeps its dropped entities, so forgetting the explicit
/// release costs a turn, not correctness.
pub struct RetainedHandle {
    id: HandleId,
    /// The map's list of dropped retained handles; taken by `release`, which then owns the
    /// release itself.
    dropped: Option<DroppedRetainedHandles>,
}

/// The ids of the retained handles dropped since the last sweep, shared between the map and
/// every handle it gave out so a `Drop` can report itself without the map.
#[derive(Clone, Default)]
struct DroppedRetainedHandles(Rc<RefCell<Vec<HandleId>>>);

impl DroppedRetainedHandles {
    fn push(&self, id: HandleId) {
        self.0.borrow_mut().push(id);
    }

    fn take(&self) -> Vec<HandleId> {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}

impl RetainedHandle {
    pub fn id(&self) -> HandleId {
        self.id
    }
}

impl Drop for RetainedHandle {
    fn drop(&mut self) {
        if let Some(dropped) = self.dropped.take() {
            dropped.push(self.id);
        }
    }
}

impl fmt::Debug for RetainedHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RetainedHandle({:?})", self.id)
    }
}

/// The entries, with the retained handles dropped since the last sweep.
pub(crate) struct HandleMap {
    entries: SlotMap<HandleId, HandleEntry>,
    dropped_retained_handles: DroppedRetainedHandles,
}

impl HandleMap {
    pub(crate) fn new() -> HandleMap {
        HandleMap {
            entries: SlotMap::with_key(),
            dropped_retained_handles: DroppedRetainedHandles::default(),
        }
    }

    pub(crate) fn create<T: 'static>(&mut self, state: T) -> Handle<T> {
        Handle::new(self.entries.insert(HandleEntry {
            type_name: type_name::<T>(),
            state: Box::new(state),
            retained: 1,
            disposed: false,
        }))
    }

    /// The owner is done with the object: Dart's `dispose`. Marks it disposed and gives up the
    /// owner's part, which removes the entry unless a [`RetainedHandle`] keeps it.
    ///
    /// # Panics
    ///
    /// If already destroyed.
    pub(crate) fn destroy(&mut self, id: HandleId) {
        let entry = self
            .entries
            .get_mut(id)
            .unwrap_or_else(|| panic!("stale handle: {id:?} was already destroyed"));
        assert!(
            !entry.disposed,
            "stale handle: {id:?} was already destroyed"
        );
        entry.disposed = true;
        self.release_retained(id);
    }

    /// Whether the entry exists: live, or destroyed but still retained.
    pub(crate) fn contains(&self, id: HandleId) -> bool {
        self.entries.contains_key(id)
    }

    /// Keeps the object past its owner's `destroy`, for the few holders that use an object
    /// across turns and rely on Dart's lingering: a cached hit-test path, the mouse tracker's
    /// remembered annotations.
    pub(crate) fn retain(&mut self, id: HandleId) -> RetainedHandle {
        self.resolve_mut(id).retained += 1;
        RetainedHandle {
            id,
            dropped: Some(self.dropped_retained_handles.clone()),
        }
    }

    /// Gives a retained handle back now; the last one removes the entry.
    pub(crate) fn release(&mut self, mut handle: RetainedHandle) {
        handle.dropped = None;
        self.release_retained(handle.id);
    }

    fn release_retained(&mut self, id: HandleId) {
        let entry = self.resolve_mut(id);
        debug_assert!(entry.retained > 0, "release without a retain: {id:?}");
        entry.retained -= 1;
        if entry.retained == 0 {
            self.entries.remove(id);
        }
    }

    /// Releases the retained handles dropped since the last sweep.
    pub(crate) fn release_dropped_retained_handles(&mut self) {
        loop {
            let dropped = self.dropped_retained_handles.take();
            if dropped.is_empty() {
                return;
            }
            for id in dropped {
                self.release_retained(id);
            }
        }
    }

    /// Whether the owner has destroyed the object while a retained handle still keeps it: it can
    /// be read and can receive events, but it is out of every tree and must not be laid out,
    /// painted or attached.
    pub(crate) fn is_disposed(&self, id: HandleId) -> bool {
        self.entries.get(id).is_some_and(|entry| entry.disposed)
    }

    /// Narrow an id. `None` if stale or the wrong type; [`get`](HandleMap::get) panics instead.
    pub(crate) fn handle<T: 'static>(&self, id: HandleId) -> Option<Handle<T>> {
        if !self.entries.get(id)?.state.is::<T>() {
            return None;
        }
        Some(Handle::new(id))
    }

    /// # Panics
    ///
    /// If the handle is stale.
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
    /// If either handle is stale, or both name the same entry.
    pub(crate) fn get_disjoint_mut<A: 'static, B: 'static>(
        &mut self,
        a: Handle<A>,
        b: Handle<B>,
    ) -> (&mut A, &mut B) {
        let [entry_a, entry_b] = self
            .entries
            .get_disjoint_mut([a.id(), b.id()])
            .unwrap_or_else(|| {
                panic!(
                    "handles {:?} and {:?} are not two live entries",
                    a.id(),
                    b.id()
                )
            });
        (
            downcast_entry_mut(entry_a, a.id()),
            downcast_entry_mut(entry_b, b.id()),
        )
    }

    fn resolve(&self, id: HandleId) -> &HandleEntry {
        self.entries
            .get(id)
            .unwrap_or_else(|| panic!("stale handle: {id:?} was destroyed"))
    }

    fn resolve_mut(&mut self, id: HandleId) -> &mut HandleEntry {
        self.entries
            .get_mut(id)
            .unwrap_or_else(|| panic!("stale handle: {id:?} was destroyed"))
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

    #[test]
    fn a_retained_object_outlives_destroy_until_released() {
        let mut handles = HandleMap::new();
        let handle = handles.create(7u32);
        let retained = handles.retain(handle.id());
        handles.destroy(handle.id());
        assert!(handles.contains(handle.id()), "kept while retained");
        assert!(handles.is_disposed(handle.id()));
        assert_eq!(*handles.get(handle), 7, "still readable");
        handles.release(retained);
        assert!(
            !handles.contains(handle.id()),
            "removed with the last release"
        );
    }

    #[test]
    fn a_dropped_retained_handle_is_released_by_the_sweep() {
        let mut handles = HandleMap::new();
        let handle = handles.create(7u32);
        let retained = handles.retain(handle.id());
        handles.destroy(handle.id());
        drop(retained);
        assert!(
            handles.contains(handle.id()),
            "the drop alone does not remove it"
        );
        handles.release_dropped_retained_handles();
        assert!(!handles.contains(handle.id()), "the sweep did");
    }

    #[test]
    fn an_unretained_object_is_removed_by_destroy() {
        let mut handles = HandleMap::new();
        let handle = handles.create(1u8);
        let retained = handles.retain(handle.id());
        handles.release(retained);
        assert!(!handles.is_disposed(handle.id()));
        handles.destroy(handle.id());
        assert!(!handles.contains(handle.id()));
    }

    #[test]
    #[should_panic(expected = "already destroyed")]
    fn destroying_twice_panics() {
        let mut handles = HandleMap::new();
        let handle = handles.create(1u8);
        handles.destroy(handle.id());
        handles.destroy(handle.id());
    }
}
