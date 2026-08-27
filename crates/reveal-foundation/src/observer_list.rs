//! Flutter counterpart: `foundation/observer_list.dart`.

use std::collections::HashSet;
use std::hash::Hash;

use indexmap::IndexMap;

/// A list optimized for the observer pattern when there are small numbers of
/// observers.
///
/// Consider using an [`ObserverList`] instead of a [`Vec`] when the number of
/// [`contains`](ObserverList::contains) calls dominates the number of
/// [`add`](ObserverList::add) and [`remove`](ObserverList::remove) calls.
///
/// This class will include in [`iter`](ObserverList::iter) each added item in
/// the order it was added, as many times as it was added.
///
/// If there will be a large number of observers, consider using
/// [`HashedObserverList`] instead. It has slightly different iteration
/// semantics, but serves a similar purpose, while being more efficient for
/// large numbers of observers.
///
/// See also:
///
///  * [`HashedObserverList`] for a list that is optimized for larger numbers of
///    observers.
pub struct ObserverList<T> {
    list: Vec<T>,
    is_dirty: bool,
    set: HashSet<T>,
}

impl<T> Default for ObserverList<T> {
    fn default() -> ObserverList<T> {
        ObserverList {
            list: Vec::new(),
            is_dirty: false,
            set: HashSet::new(),
        }
    }
}

impl<T: Eq + Hash> ObserverList<T> {
    /// Creates an empty [`ObserverList`].
    pub fn new() -> ObserverList<T> {
        ObserverList::default()
    }

    /// Adds an item to the end of this list.
    ///
    /// This operation has constant time complexity.
    pub fn add(&mut self, item: T) {
        self.is_dirty = true;
        self.list.push(item);
    }

    /// Removes an item from the list.
    ///
    /// This is O(N) in the number of items in the list.
    ///
    /// Returns whether the item was present in the list.
    pub fn remove(&mut self, item: &T) -> bool {
        let Some(index) = self.list.iter().position(|entry| entry == item) else {
            return false;
        };
        self.list.remove(index);
        self.is_dirty = true;
        self.set.clear(); // Clear the set so that we don't leak items.
        true
    }

    /// Removes all items from the [`ObserverList`].
    pub fn clear(&mut self) {
        self.is_dirty = false;
        self.list.clear();
        self.set.clear();
    }

    /// Whether the list contains `element`.
    pub fn contains(&mut self, element: &T) -> bool
    where
        T: Clone,
    {
        if self.list.len() < 3 {
            return self.list.contains(element);
        }

        if self.is_dirty {
            self.set.extend(self.list.iter().cloned());
            self.is_dirty = false;
        }

        self.set.contains(element)
    }

    /// The items in this list, in the order they were added, as many times as
    /// they were added.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.list.iter()
    }

    /// Whether the list has no items.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Whether the list has any items.
    pub fn is_not_empty(&self) -> bool {
        !self.list.is_empty()
    }

    /// Creates a [`Vec`] containing the elements of the [`ObserverList`].
    pub fn to_list(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.list.clone()
    }
}

impl<'a, T: Eq + Hash> IntoIterator for &'a ObserverList<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A list optimized for the observer pattern, but for larger numbers of observers.
///
/// For small numbers of observers (e.g. less than 10), use [`ObserverList`]
/// instead.
///
/// The iteration semantics of this class are slightly different from
/// [`ObserverList`]. This class will only return an item once in
/// [`iter`](HashedObserverList::iter), no matter how many times it was added,
/// although it does require that an item be removed as many times as it was
/// added for it to stop appearing in the iteration. It will return them in the
/// order the first instance of an item was originally added.
///
/// See also:
///
///  * [`ObserverList`] for a list that is fast for small numbers of observers.
pub struct HashedObserverList<T> {
    map: IndexMap<T, usize>,
}

impl<T> Default for HashedObserverList<T> {
    fn default() -> HashedObserverList<T> {
        HashedObserverList {
            map: IndexMap::new(),
        }
    }
}

impl<T: Eq + Hash> HashedObserverList<T> {
    /// Creates an empty [`HashedObserverList`].
    pub fn new() -> HashedObserverList<T> {
        HashedObserverList::default()
    }

    /// Adds an item to the end of this list.
    ///
    /// This has constant time complexity.
    pub fn add(&mut self, item: T) {
        *self.map.entry(item).or_insert(0) += 1;
    }

    /// Removes an item from the list.
    ///
    /// Returns whether the item was present in the list.
    pub fn remove(&mut self, item: &T) -> bool {
        let Some(&value) = self.map.get(item) else {
            return false;
        };
        if value == 1 {
            self.map.shift_remove(item);
        } else {
            self.map[item] = value - 1;
        }
        true
    }

    /// Removes all items from the [`HashedObserverList`].
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Whether the list contains `element`.
    pub fn contains(&self, element: &T) -> bool {
        self.map.contains_key(element)
    }

    /// The items in this list, each once, in the order the first instance of
    /// each was added.
    pub fn iter(&self) -> indexmap::map::Keys<'_, T, usize> {
        self.map.keys()
    }

    /// Whether the list has no items.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Whether the list has any items.
    pub fn is_not_empty(&self) -> bool {
        !self.map.is_empty()
    }

    /// Creates a [`Vec`] containing the elements of the [`HashedObserverList`].
    pub fn to_list(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.map.keys().cloned().collect()
    }
}

impl<'a, T: Eq + Hash> IntoIterator for &'a HashedObserverList<T> {
    type Item = &'a T;
    type IntoIter = indexmap::map::Keys<'a, T, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_list() {
        let mut list = ObserverList::<i32>::new();
        for i in 0..10 {
            list.add(i);
        }
        let mut iterator = list.iter();
        let mut i = 0;
        while i < 10 {
            if let Some(&current) = iterator.next() {
                assert_eq!(current, i);
            }
            i += 1;
        }
        for i in (0..10).rev() {
            assert!(list.remove(&i));
            let mut iterator = list.iter();
            let mut j = 0;
            while j < i {
                if let Some(&current) = iterator.next() {
                    assert_eq!(current, j);
                }
                j += 1;
            }
        }
    }

    #[test]
    fn hashed_observer_list() {
        let mut list = HashedObserverList::<i32>::new();
        for i in 0..10 {
            list.add(i);
        }
        let mut iterator = list.iter();
        let mut i = 0;
        while i < 10 {
            if let Some(&current) = iterator.next() {
                assert_eq!(current, i);
            }
            i += 1;
        }
        for i in (0..10).rev() {
            assert!(list.remove(&i));
            let mut iterator = list.iter();
            let mut j = 0;
            while j < i {
                if let Some(&current) = iterator.next() {
                    assert_eq!(current, j);
                }
                j += 1;
            }
        }
        list.add(0);
        for _ in 0..10 {
            list.add(1);
        }
        list.add(2);
        for (i, &current) in list.iter().enumerate() {
            assert_eq!(current, i as i32);
            assert!(i < 3);
        }
        for i in (0..3).rev() {
            assert!(list.remove(&i));
            for (j, &current) in list.iter().enumerate() {
                assert_eq!(current, if i != 0 { j as i32 } else { 1 });
                assert!(j < 3);
            }
        }
        for (j, &current) in list.iter().enumerate() {
            assert_eq!(current, 1);
            assert_eq!(j, 0);
        }
        assert!(!list.is_empty());
        let mut iterator = list.iter();
        assert_eq!(iterator.next(), Some(&1));
        for _ in 0..9 {
            assert!(list.remove(&1));
        }
        assert!(list.is_empty());
    }

    #[test]
    fn an_item_is_included_as_many_times_as_it_was_added() {
        let mut list = ObserverList::new();
        list.add(1);
        list.add(2);
        list.add(1);

        assert_eq!(list.iter().copied().collect::<Vec<_>>(), [1, 2, 1]);
        assert!(list.remove(&1));
        assert_eq!(list.iter().copied().collect::<Vec<_>>(), [2, 1]);
    }

    #[test]
    fn contains_is_right_on_both_sides_of_the_set_threshold() {
        let mut list = ObserverList::new();
        for item in 0..2 {
            list.add(item);
        }
        assert!(list.contains(&1));
        assert!(!list.contains(&9));

        for item in 2..5 {
            list.add(item);
        }
        assert!(list.contains(&1));
        assert!(!list.contains(&9));
    }

    #[test]
    fn removing_invalidates_the_set_cache() {
        let mut list = ObserverList::new();
        for item in 0..5 {
            list.add(item);
        }
        assert!(list.contains(&4));

        assert!(list.remove(&4));
        assert!(!list.contains(&4));
        assert!(list.contains(&3));

        assert!(!list.remove(&4));
    }

    #[test]
    fn clearing_empties_the_list_and_the_set_cache() {
        let mut list = ObserverList::new();
        for item in 0..5 {
            list.add(item);
        }
        assert!(list.contains(&4));

        list.clear();
        assert!(list.is_empty());
        assert!(!list.is_not_empty());
        assert!(!list.contains(&4));

        for item in 0..3 {
            list.add(item);
        }
        assert!(!list.contains(&4));
        assert!(list.contains(&0));
    }

    #[test]
    fn a_snapshot_lets_a_listener_mutate_the_list_during_dispatch() {
        let mut list = ObserverList::new();
        list.add(1);
        list.add(2);

        let mut dispatched = Vec::new();
        for observer in list.to_list() {
            dispatched.push(observer);
            list.remove(&observer);
        }

        assert_eq!(dispatched, [1, 2]);
        assert!(list.is_empty());
    }

    #[test]
    fn a_removed_listener_is_skipped_after_the_snapshot() {
        let mut list = HashedObserverList::new();
        list.add(1);
        list.add(2);

        let mut called = Vec::new();
        for observer in list.to_list() {
            if list.contains(&observer) {
                called.push(observer);
                if observer == 1 {
                    list.remove(&2);
                }
            }
        }

        assert_eq!(called, [1]);
        assert!(list.contains(&1));
        assert!(!list.contains(&2));
    }

    #[test]
    fn a_listener_added_during_dispatch_is_not_called() {
        let mut list = HashedObserverList::new();
        list.add(1);
        list.add(2);

        let mut called = Vec::new();
        for observer in list.to_list() {
            if list.contains(&observer) {
                called.push(observer);
                list.add(99);
            }
        }

        assert_eq!(called, [1, 2]);
        assert!(list.contains(&99));
    }

    #[test]
    fn a_hashed_item_is_included_once_but_must_be_removed_as_often_as_added() {
        let mut list = HashedObserverList::new();
        list.add(1);
        list.add(1);
        list.add(2);

        assert_eq!(list.iter().copied().collect::<Vec<_>>(), [1, 2]);

        assert!(list.remove(&1));
        assert!(list.contains(&1));
        assert!(list.remove(&1));
        assert!(!list.contains(&1));
        assert!(!list.remove(&1));

        assert_eq!(list.iter().copied().collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn hashed_iteration_follows_first_insertion_order() {
        let mut list = HashedObserverList::new();
        for item in [30, 10, 20] {
            list.add(item);
        }
        list.add(30);

        assert_eq!(list.to_list(), [30, 10, 20]);

        assert!(list.remove(&10));
        assert_eq!(list.to_list(), [30, 20]);

        assert!(list.remove(&30));
        assert_eq!(list.to_list(), [30, 20]);
        assert!(list.remove(&30));
        assert_eq!(list.to_list(), [20]);

        let mut list = HashedObserverList::new();
        for item in [30, 10, 20] {
            list.add(item);
        }
        assert!(list.remove(&30));
        assert_eq!(list.to_list(), [10, 20]);
    }

    #[test]
    fn clearing_a_hashed_list_drops_every_registration() {
        let mut list = HashedObserverList::new();
        list.add(1);
        list.add(1);

        list.clear();
        assert!(list.is_empty());
        assert!(!list.is_not_empty());
        assert!(!list.contains(&1));
    }
}
