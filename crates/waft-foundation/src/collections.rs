//! Flutter counterpart: `foundation/collections.dart` (the equality checks, the searches and the
//! sorts).
//!
//! Flutter's `null` collection is `None`: [`list_equals`], [`set_equals`] and [`map_equals`]
//! take an `Option` of each side, and two `None`s are equal.
//!
//! [`merge_sort`] is Flutter's stable merge sort: it keeps equal elements in the order they
//! started in, and it does not require its comparator to be a consistent total order, which
//! Flutter's screen-order comparator is not. Rust's `sort_by` may panic on such a comparator;
//! a selection delegate sorts selectables by where they are on screen, so it uses this.
//!
//! [`merge_sort_range`] is Dart's `mergeSort(start:, end:)`; the insertion sort it falls back
//! to below 32 elements is private, as the helpers it merges with.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Limit below which merge sort defaults to insertion sort.
const K_MERGE_SORT_LIMIT: usize = 32;

/// Compares two sets for element-by-element equality.
///
/// Two `None`s are equal, and a `None` is never equal to a set. Two sets are equal when they
/// have the same length and every element of one is in the other; order is not compared.
pub fn set_equals<T: Eq + Hash>(a: Option<&HashSet<T>>, b: Option<&HashSet<T>>) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return a.is_none() && b.is_none();
    };
    if a.len() != b.len() {
        return false;
    }
    if std::ptr::eq(a, b) {
        return true;
    }
    a.iter().all(|value| b.contains(value))
}

/// Compares two lists for element-by-element equality.
///
/// Two `None`s are equal, and a `None` is never equal to a list. Two lists are equal when they
/// have the same length and the same elements in the same order.
pub fn list_equals<T: PartialEq>(a: Option<&[T]>, b: Option<&[T]>) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return a.is_none() && b.is_none();
    };
    if a.len() != b.len() {
        return false;
    }
    if std::ptr::eq(a, b) {
        return true;
    }
    a.iter().zip(b).all(|(x, y)| x == y)
}

/// Compares two maps for element-by-element equality.
///
/// Two `None`s are equal, and a `None` is never equal to a map. Two maps are equal when they
/// have the same length, and every key of one is a key of the other with an equal value.
pub fn map_equals<K: Eq + Hash, V: PartialEq>(
    a: Option<&HashMap<K, V>>,
    b: Option<&HashMap<K, V>>,
) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return a.is_none() && b.is_none();
    };
    if a.len() != b.len() {
        return false;
    }
    if std::ptr::eq(a, b) {
        return true;
    }
    a.iter().all(|(key, value)| b.get(key) == Some(value))
}

/// Returns the position of `value` in `sorted_list`, or `-1` if it does not contain it.
///
/// The list must already be ordered; Dart's contract is otherwise the same.
pub fn binary_search<T: Ord>(sorted_list: &[T], value: &T) -> isize {
    let mut min = 0;
    let mut max = sorted_list.len();
    while min < max {
        let mid = min + ((max - min) >> 1);
        match sorted_list[mid].cmp(value) {
            Ordering::Equal => return mid as isize,
            Ordering::Less => min = mid + 1,
            Ordering::Greater => max = mid,
        }
    }
    -1
}

/// Sorts `list` with Flutter's merge sort, which is stable.
///
/// `compare` need not be a consistent total order: the sort only requires that equal elements
/// compare [`Ordering::Equal`] to each other, as the merge compares two elements at a time and
/// keeps the first on a tie.
pub fn merge_sort<T: Clone>(list: &mut [T], compare: impl FnMut(&T, &T) -> Ordering) {
    let end = list.len();
    merge_sort_range(list, 0, end, compare);
}

/// Sorts `list[start..end]` with Flutter's merge sort; Dart's `mergeSort(start:, end:)`.
pub fn merge_sort_range<T: Clone>(
    list: &mut [T],
    start: usize,
    end: usize,
    mut compare: impl FnMut(&T, &T) -> Ordering,
) {
    debug_assert!(start <= end && end <= list.len());
    let length = end - start;
    if length < 2 {
        return;
    }
    if length < K_MERGE_SORT_LIMIT {
        insertion_sort(list, &mut compare, start, end);
        return;
    }
    // Special case the first split instead of directly calling `merge_sort_into`, because it
    // requires its target to be different from its source, and it requires extra space of the
    // same size as the list to sort. This split allows only half as much extra space, and it
    // ends up in the original place.
    let middle = start + ((end - start) >> 1);
    let first_length = middle - start;
    let second_length = end - middle;
    // `second_length` is always the same as `first_length`, or one greater.
    let mut scratch_space = vec![list[start].clone(); second_length];
    merge_sort_into(
        &mut list[middle..end],
        &mut compare,
        0,
        second_length,
        &mut scratch_space,
        0,
    );
    let first_target = end - first_length;
    {
        // `list[first_target..end]` is free to sort into: the second half was copied to the
        // scratch space above.
        let (lower, upper) = list.split_at_mut(middle);
        merge_sort_into(
            &mut lower[start..middle],
            &mut compare,
            0,
            first_length,
            &mut upper[(first_target - middle)..(first_target - middle + first_length)],
            0,
        );
    }
    merge_into_list(
        &mut compare,
        list,
        first_target,
        end,
        &scratch_space,
        0,
        second_length,
        start,
    );
}

/// Sort a list between `start` (inclusive) and `end` (exclusive) using insertion sort.
///
/// This insertion sort is stable: equal elements end up in the same order as they started in.
fn insertion_sort<T: Clone>(
    list: &mut [T],
    compare: &mut impl FnMut(&T, &T) -> Ordering,
    start: usize,
    end: usize,
) {
    for pos in (start + 1)..end {
        let element = list[pos].clone();
        let mut min = start;
        let mut max = pos;
        while min < max {
            let mid = min + ((max - min) >> 1);
            if compare(&element, &list[mid]) == Ordering::Less {
                max = mid;
            } else {
                min = mid + 1;
            }
        }
        // `list.setRange(min + 1, pos + 1, list, min)`.
        for index in (min..pos).rev() {
            list[index + 1] = list[index].clone();
        }
        list[min] = element;
    }
}

/// Performs an insertion sort into a potentially different list than the one containing the
/// original values.
///
/// It will work in-place as well, when the target range does not overlap the source.
fn moving_insertion_sort<T: Clone>(
    source: &[T],
    compare: &mut impl FnMut(&T, &T) -> Ordering,
    start: usize,
    end: usize,
    target: &mut [T],
    target_offset: usize,
) {
    let length = end - start;
    if length == 0 {
        return;
    }
    target[target_offset] = source[start].clone();
    for i in 1..length {
        let element = source[start + i].clone();
        let mut min = target_offset;
        let mut max = target_offset + i;
        while min < max {
            let mid = min + ((max - min) >> 1);
            if compare(&element, &target[mid]) == Ordering::Less {
                max = mid;
            } else {
                min = mid + 1;
            }
        }
        // `target.setRange(min + 1, targetOffset + i + 1, target, min)`.
        for index in (min..(target_offset + i)).rev() {
            target[index + 1] = target[index].clone();
        }
        target[min] = element;
    }
}

/// Sorts `source[start..end]` into `target` at `target_offset`.
///
/// The target must be able to contain the range after `target_offset`, and the ranges are
/// disjoint: a caller that sorts into the same list splits it first.
fn merge_sort_into<T: Clone>(
    source: &mut [T],
    compare: &mut impl FnMut(&T, &T) -> Ordering,
    start: usize,
    end: usize,
    target: &mut [T],
    target_offset: usize,
) {
    let length = end - start;
    if length < K_MERGE_SORT_LIMIT {
        moving_insertion_sort(
            &source[start..end],
            &mut *compare,
            0,
            length,
            &mut target[target_offset..],
            0,
        );
        return;
    }
    let middle = start + (length >> 1);
    let first_length = middle - start;
    let second_length = end - middle;
    // Here `second_length >= first_length` (they differ by at most one).
    let target_middle = target_offset + first_length;
    // Sort the second half into the end of the target area.
    merge_sort_into(source, &mut *compare, middle, end, target, target_middle);
    // Sort the first half into the end of the source area, which the second half left free.
    {
        let (lower, upper) = source.split_at_mut(middle);
        merge_sort_into(
            &mut lower[start..middle],
            &mut *compare,
            0,
            first_length,
            &mut upper[..first_length],
            0,
        );
    }
    // Merge the two parts into the target area.
    merge_slices_into_target(
        &mut *compare,
        &source[middle..(middle + first_length)],
        0,
        first_length,
        target,
        target_middle,
        target_middle + second_length,
        target_offset,
    );
}

/// Merges two ranges into `list` at `target_offset`.
///
/// The first range is in `list`; the target may overlap it, and the merge writes forward from
/// the target offset while both ranges are read forward, so it never overwrites an element it
/// has not read yet.
///
/// For equal objects, elements from the first list are always preferred. This allows the merge
/// to be stable if the first list contains elements that started out earlier than the ones in
/// the second list.
#[allow(clippy::too_many_arguments)]
fn merge_into_list<T: Clone>(
    compare: &mut impl FnMut(&T, &T) -> Ordering,
    list: &mut [T],
    first_start: usize,
    first_end: usize,
    second: &[T],
    second_start: usize,
    second_end: usize,
    target_offset: usize,
) {
    // No empty lists reach here.
    debug_assert!(first_start < first_end);
    debug_assert!(second_start < second_end);
    let mut cursor1 = first_start;
    let mut cursor2 = second_start;
    let mut target = target_offset;
    let mut first_element = list[cursor1].clone();
    cursor1 += 1;
    let mut second_element = second[cursor2].clone();
    cursor2 += 1;
    loop {
        if compare(&first_element, &second_element) != Ordering::Greater {
            list[target] = first_element;
            target += 1;
            if cursor1 == first_end {
                // Flushing the second list after the loop.
                break;
            }
            first_element = list[cursor1].clone();
            cursor1 += 1;
        } else {
            list[target] = second_element;
            target += 1;
            if cursor2 != second_end {
                second_element = second[cursor2].clone();
                cursor2 += 1;
                continue;
            }
            // The second list emptied first; flush the first list here.
            list[target] = first_element;
            target += 1;
            for index in 0..(first_end - cursor1) {
                list[target + index] = list[cursor1 + index].clone();
            }
            return;
        }
    }
    // The first list emptied first; reached by the break above.
    list[target] = second_element;
    target += 1;
    // The source and the target are the same list, so this cannot be `clone_from_slice`.
    #[allow(clippy::manual_memcpy)]
    for index in 0..(second_end - cursor2) {
        list[target + index] = second[cursor2 + index].clone();
    }
}

/// Merges a range of `first` and a range of `target` into `target` at `target_offset`.
///
/// The second range is in `target` and may overlap the target range; the copy of its tail is
/// forward, from index `second_start` onward, into an earlier target offset.
///
/// For equal objects, elements from the first range are always preferred, as in
/// [`merge_into_list`].
#[allow(clippy::too_many_arguments)]
fn merge_slices_into_target<T: Clone>(
    compare: &mut impl FnMut(&T, &T) -> Ordering,
    first: &[T],
    first_start: usize,
    first_end: usize,
    target: &mut [T],
    second_start: usize,
    second_end: usize,
    target_offset: usize,
) {
    // No empty lists reach here.
    debug_assert!(first_start < first_end);
    debug_assert!(second_start < second_end);
    let mut cursor1 = first_start;
    let mut cursor2 = second_start;
    let mut target_index = target_offset;
    let mut first_element = first[cursor1].clone();
    cursor1 += 1;
    let mut second_element = target[cursor2].clone();
    cursor2 += 1;
    loop {
        if compare(&first_element, &second_element) != Ordering::Greater {
            target[target_index] = first_element;
            target_index += 1;
            if cursor1 == first_end {
                break;
            }
            first_element = first[cursor1].clone();
            cursor1 += 1;
        } else {
            target[target_index] = second_element;
            target_index += 1;
            if cursor2 != second_end {
                second_element = target[cursor2].clone();
                cursor2 += 1;
                continue;
            }
            target[target_index] = first_element;
            target_index += 1;
            // The first range and the target may be the same list; `clone_from_slice` cannot
            // borrow both.
            #[allow(clippy::manual_memcpy)]
            for index in 0..(first_end - cursor1) {
                target[target_index + index] = first[cursor1 + index].clone();
            }
            return;
        }
    }
    target[target_index] = second_element;
    target_index += 1;
    for index in 0..(second_end - cursor2) {
        target[target_index + index] = target[cursor2 + index].clone();
    }
}

#[cfg(test)]
mod tests {
    //! `test/foundation/collections_test.dart`: its equality, `binarySearch` and `MergeSort*`
    //! cases.

    use super::*;

    /// Dart's `Random()` with a fixed seed, so a failing case repeats.
    struct Random(u64);

    impl Random {
        fn new(seed: u64) -> Random {
            Random(seed)
        }

        fn next(&mut self, bound: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) as usize) % bound
        }
    }

    /// The test's `OrderedComparable`: compares by `id` only, so ties keep their `order`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct OrderedComparable {
        id: i32,
        order: i32,
    }

    impl Ord for OrderedComparable {
        fn cmp(&self, other: &Self) -> Ordering {
            self.id.cmp(&other.id)
        }
    }

    impl PartialOrd for OrderedComparable {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    // collections_test.dart 'listEquals'.
    #[test]
    fn list_equals_compares_length_order_and_elements() {
        let list_a = vec![1, 2, 3];
        let list_b = vec![1, 2, 3];
        let list_c = vec![1, 2];
        let list_d = vec![3, 2, 1];

        assert!(list_equals::<i32>(None, None));
        assert!(!list_equals(Some(&list_a), None));
        assert!(!list_equals(None, Some(&list_b)));
        assert!(list_equals(Some(&list_a), Some(&list_a)));
        assert!(list_equals(Some(&list_a), Some(&list_b)));
        assert!(!list_equals(Some(&list_a), Some(&list_c)));
        assert!(!list_equals(Some(&list_a), Some(&list_d)));
    }

    // collections_test.dart 'setEquals'.
    #[test]
    fn set_equals_ignores_order() {
        let set_a = HashSet::from([1, 2, 3]);
        let set_b = HashSet::from([1, 2, 3]);
        let set_c = HashSet::from([1, 2]);
        let set_d = HashSet::from([3, 2, 1]);

        assert!(set_equals::<i32>(None, None));
        assert!(!set_equals(Some(&set_a), None));
        assert!(!set_equals(None, Some(&set_b)));
        assert!(set_equals(Some(&set_a), Some(&set_a)));
        assert!(set_equals(Some(&set_a), Some(&set_b)));
        assert!(!set_equals(Some(&set_a), Some(&set_c)));
        assert!(set_equals(Some(&set_a), Some(&set_d)));
    }

    // collections_test.dart 'mapEquals'.
    #[test]
    fn map_equals_compares_keys_and_values() {
        let map_a = HashMap::from([(1, 1), (2, 2), (3, 3)]);
        let map_b = HashMap::from([(1, 1), (2, 2), (3, 3)]);
        let map_c = HashMap::from([(1, 1), (2, 2)]);
        let map_d = HashMap::from([(3, 3), (2, 2), (1, 1)]);
        let map_e = HashMap::from([(3, 1), (2, 2), (1, 3)]);

        assert!(map_equals::<i32, i32>(None, None));
        assert!(!map_equals(Some(&map_a), None));
        assert!(!map_equals(None, Some(&map_b)));
        assert!(map_equals(Some(&map_a), Some(&map_a)));
        assert!(map_equals(Some(&map_a), Some(&map_b)));
        assert!(!map_equals(Some(&map_a), Some(&map_c)));
        assert!(map_equals(Some(&map_a), Some(&map_d)));
        assert!(!map_equals(Some(&map_a), Some(&map_e)));
    }

    // collections_test.dart 'binarySearch'.
    #[test]
    fn binary_search_finds_the_value_or_minus_one() {
        let items = [1, 2, 3];
        assert_eq!(binary_search(&items, &1), 0);
        assert_eq!(binary_search(&items, &2), 1);
        assert_eq!(binary_search(&items, &3), 2);
        assert_eq!(binary_search(&items, &12), -1);
    }

    // collections_test.dart 'MergeSortRandom'.
    #[test]
    fn merge_sort_orders_random_lists() {
        for i in 0..250usize {
            // Expect some equal elements.
            let mut random = Random::new(i as u64 + 1);
            let mut list: Vec<i32> = (0..i).map(|_| random.next(i.max(1)) as i32).collect();
            merge_sort(&mut list, |a, b| a.cmp(b));
            for j in 1..i {
                assert!(
                    list[j - 1] <= list[j],
                    "element {j} of {list:?} is smaller than the one before it"
                );
            }
        }
    }

    // collections_test.dart 'MergeSortPreservesOrder'.
    #[test]
    fn merge_sort_preserves_the_order_of_equal_elements() {
        // A small case where only the insertion sort runs, larger ones where insertion runs
        // inside the merge, and larger cases with several splits, around a power of two.
        for size in [8usize, 50, 511, 512, 513] {
            let mut random = Random::new(size as u64);
            let mut list: Vec<OrderedComparable> = (0..size)
                .map(|i| OrderedComparable {
                    id: random.next(size >> 2) as i32,
                    order: i as i32,
                })
                .collect();
            merge_sort(&mut list, |a, b| a.cmp(b));
            let mut previous = list[0];
            for next in &list[1..] {
                assert!(previous.id <= next.id);
                if next.id == previous.id {
                    assert!(previous.order <= next.order);
                }
                previous = *next;
            }

            // Reverse compare on part of the list.
            let copy = list.clone();
            let min = size >> 2;
            let max = size - min;
            merge_sort_range(&mut list, min, max, |a, b| b.cmp(a));
            let mut previous = list[min];
            for next in &list[(min + 1)..max] {
                assert!(previous.id >= next.id);
                if next.id == previous.id {
                    assert!(previous.order <= next.order);
                }
                previous = *next;
            }
            // `OrderedComparable` compares by id, so this means the parts before `min` and
            // after `max` did not change at all.
            assert_eq!(&list[..min], &copy[..min]);
            assert_eq!(&list[max..], &copy[max..]);
        }
    }

    // collections_test.dart 'MergeSortSpecialCases'.
    #[test]
    fn merge_sort_orders_all_equal_and_reversed_lists() {
        for size in [511usize, 512, 513] {
            // All equal.
            let mut list: Vec<OrderedComparable> = (0..size)
                .map(|i| OrderedComparable {
                    id: 0,
                    order: i as i32,
                })
                .collect();
            merge_sort(&mut list, |a, b| a.cmp(b));
            for (i, element) in list.iter().enumerate() {
                assert_eq!(element.order, i as i32);
            }

            // All but one equal, first.
            list[0] = OrderedComparable { id: 1, order: 0 };
            for (i, element) in list.iter_mut().enumerate().skip(1) {
                *element = OrderedComparable {
                    id: 0,
                    order: i as i32,
                };
            }
            merge_sort(&mut list, |a, b| a.cmp(b));
            for (i, element) in list.iter().enumerate().take(size - 1) {
                assert_eq!(element.order, i as i32 + 1);
            }
            assert_eq!(list[size - 1].order, 0);

            // All but one equal, last.
            for (i, element) in list.iter_mut().enumerate().take(size - 1) {
                *element = OrderedComparable {
                    id: 0,
                    order: i as i32,
                };
            }
            list[size - 1] = OrderedComparable {
                id: -1,
                order: size as i32 - 1,
            };
            merge_sort(&mut list, |a, b| a.cmp(b));
            assert_eq!(list[0].order, size as i32 - 1);
            for (i, element) in list.iter().enumerate().skip(1) {
                assert_eq!(element.order, i as i32 - 1);
            }

            // Reversed.
            for (i, element) in list.iter_mut().enumerate() {
                *element = OrderedComparable {
                    id: (size - 1 - i) as i32,
                    order: i as i32,
                };
            }
            merge_sort(&mut list, |a, b| a.cmp(b));
            for (i, element) in list.iter().enumerate() {
                assert_eq!(element.id, i as i32);
                assert_eq!(element.order, (size - 1 - i) as i32);
            }
        }
    }
}
