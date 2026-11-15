// Copyright 2011-2012, the Dart project authors.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//     * Redistributions of source code must retain the above copyright
//       notice, this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above
//       copyright notice, this list of conditions and the following
//       disclaimer in the documentation and/or other materials provided
//       with the distribution.
//     * Neither the name of Google LLC nor the names of its
//       contributors may be used to endorse or promote products derived
//       from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//
// Ported from `lib/_internal/vm/lib/math_patch.dart`, `lib/_internal/vm/lib/double.dart`,
// `lib/core/num.dart` and `lib/internal/sort.dart` of the Dart SDK at revision
// e5add7a25b32066ee1c83cef3d9ab0c9d018bb65 (3.15.0-139.0.dev).

//! The parts of the Dart core libraries that material_color_utilities calls and Rust's
//! standard library answers differently: `math.max` and `math.min` on doubles,
//! `double.compareTo`, the `%` of doubles, `double.round`, `floor` and `ceil` to an `int`,
//! `List.sort`, and the seeded `math.Random`.
//!
//! They follow the Dart VM, the runtime the package's expected values come from. Keep them
//! bit-for-bit with it: a quantizer's clusters and a tone at a NaN edge depend on it.

use std::cmp::Ordering;

/// Dart's `math.max` on doubles.
///
/// A NaN argument gives NaN, and `max(-0.0, 0.0)` is `0.0`, where Rust's `f64::max`
/// returns the other argument for a NaN.
pub(crate) fn max(a: f64, b: f64) -> f64 {
    if a > b {
        return a;
    }
    if a < b {
        return b;
    }
    if a == 0.0 {
        // a is either 0.0 or -0.0. b is either 0.0, -0.0, or NaN.
        // The following returns 0.0 if either a or b is 0.0, and it
        // returns NaN if b is NaN.
        return a + b;
    }
    // Check for NaN.
    if b.is_nan() {
        return b;
    }
    a
}

/// Dart's `math.min` on doubles.
///
/// A NaN argument gives NaN, and `min(-0.0, 0.0)` is `-0.0`, where Rust's `f64::min`
/// returns the other argument for a NaN.
pub(crate) fn min(a: f64, b: f64) -> f64 {
    if a > b {
        return b;
    }
    if a < b {
        return a;
    }
    if a == 0.0 {
        // a is either 0.0 or -0.0. b is either 0.0, -0.0 or NaN.
        // The following returns -0.0 if either a or b is -0.0, and it
        // returns NaN if b is NaN.
        return (a + b) * a * b;
    }
    // Check for NaN and b == -0.0.
    if a == 0.0 && b.is_sign_negative() || b.is_nan() {
        return b;
    }
    a
}

/// Dart's `double.compareTo`: `-0.0` orders before `0.0`, and NaN after every other
/// value and equal to itself.
pub(crate) fn compare_to(this: f64, other: f64) -> Ordering {
    if this < other {
        Ordering::Less
    } else if this > other {
        Ordering::Greater
    } else if this == other {
        if this == 0.0 {
            let this_is_negative = this.is_sign_negative();
            let other_is_negative = other.is_sign_negative();
            if this_is_negative == other_is_negative {
                return Ordering::Equal;
            }
            return if this_is_negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        Ordering::Equal
    } else if this.is_nan() {
        if other.is_nan() {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    } else {
        // Other is NaN.
        Ordering::Less
    }
}

/// Dart's `%` on doubles: the Euclidean modulo, whose sign is always positive.
///
/// A zero remainder is `0.0` where Rust's `%` keeps the dividend's sign and gives `-0.0`, and a
/// negative remainder moves up by `|other|`.
pub(crate) fn modulo(this: f64, other: f64) -> f64 {
    let remainder = this % other;
    if remainder == 0.0 {
        return 0.0;
    }
    if remainder < 0.0 {
        return remainder + other.abs();
    }
    remainder
}

/// Dart's `double.round()`: half away from zero, to an `int`. A value beyond 64 bits clamps,
/// and NaN or an infinity panics where Dart throws `UnsupportedError`.
pub(crate) fn round(value: f64) -> i64 {
    to_int(value.round())
}

/// Dart's `double.floor()` to an `int`, as [`round`] treats the edges.
pub(crate) fn floor(value: f64) -> i64 {
    to_int(value.floor())
}

/// Dart's `double.ceil()` to an `int`, as [`round`] treats the edges.
pub(crate) fn ceil(value: f64) -> i64 {
    to_int(value.ceil())
}

fn to_int(integral: f64) -> i64 {
    assert!(
        integral.is_finite(),
        "Unsupported operation: Infinity or NaN toInt"
    );
    integral as i64
}

/// Dart's `Random(seed)` on the VM: the same seed gives the same sequence as Dart's.
pub(crate) struct Random {
    state: u64,
}

impl Random {
    // Constants used by the algorithm.
    const POW2_32: u64 = 1 << 32;

    /// Dart `Random(seed)`.
    pub(crate) fn new(seed: i64) -> Random {
        let mut random = Random {
            state: Self::setup_seed(seed as u64),
        };
        // Crank a couple of times to distribute the seed bits a bit further.
        random.next_state();
        random.next_state();
        random.next_state();
        random.next_state();
        random
    }

    // The algorithm used here is Multiply with Carry (MWC) with a Base b = 2^32.
    // http://en.wikipedia.org/wiki/Multiply-with-carry
    // The constant A is selected from "Numerical Recipes 3rd Edition" p.348 B1.
    fn next_state(&mut self) {
        const A: u64 = 0xffffda61;
        let state_lo = self.state & 0xFFFFFFFF;
        let state_hi = self.state >> 32;
        self.state = A.wrapping_mul(state_lo).wrapping_add(state_hi);
    }

    /// Dart `Random.nextInt`: a value in `0..max`.
    pub(crate) fn next_int(&mut self, max: u64) -> u64 {
        const LIMIT: u64 = 0x3FFFFFFF;
        assert!(
            max > 0 && (max <= LIMIT || max <= Self::POW2_32),
            "max must be positive and <= 2^32, got {max}"
        );
        if max.is_power_of_two() {
            // Fast case for powers of two.
            self.next_state();
            return self.state & 0xFFFFFFFF & (max - 1);
        }

        loop {
            self.next_state();
            let rnd32 = self.state & 0xFFFFFFFF;
            let result = rnd32 % max;
            if rnd32 - result + max <= Self::POW2_32 {
                return result;
            }
        }
    }

    // Thomas Wang 64-bit mix.
    // http://www.concentric.net/~Ttwang/tech/inthash.htm
    // via. http://web.archive.org/web/20071223173210/http://www.concentric.net/~Ttwang/tech/inthash.htm
    fn setup_seed(mut n: u64) -> u64 {
        n = (!n).wrapping_add(n << 21); // n = (n << 21) - n - 1;
        n ^= n >> 24;
        n = n.wrapping_mul(265); // n = (n + (n << 3)) + (n << 8);
        n ^= n >> 14;
        n = n.wrapping_mul(21); // n = (n + (n << 2)) + (n << 4);
        n ^= n >> 28;
        n = n.wrapping_add(n << 31);
        if n == 0 {
            n = 0x5a17;
        }
        n
    }
}

/// When a list has less than [`INSERTION_SORT_THRESHOLD`] elements it will be sorted by an
/// insertion sort.
const INSERTION_SORT_THRESHOLD: isize = 32;

/// Dart's `List.sort` on the VM: `Sort.sort`, an insertion sort up to 33 elements and the
/// dual-pivot quicksort of Vladimir Yaroslavskiy's paper above, with some improvements copied
/// from Android's implementation.
///
/// It is not stable, and it puts equal elements in the order Dart's does: a ranking with ties
/// depends on it. `compare` is the element type's Dart `compareTo`, consistent or not; an
/// inconsistent one (a NaN that compares greater than itself) gives Dart's order, never a
/// panic of Rust's own sorts.
pub(crate) fn sort<E: Clone>(a: &mut [E], compare: impl Fn(&E, &E) -> Ordering) {
    let right = a.len() as isize - 1;
    Sort { a, compare }.do_sort(0, right);
}

/// The list [`sort`] sorts and its `compare`. Indices are Dart's `int`s: a range's `right`
/// can be one below its `left`.
struct Sort<'a, E, C> {
    a: &'a mut [E],
    compare: C,
}

impl<E: Clone, C: Fn(&E, &E) -> Ordering> Sort<'_, E, C> {
    fn at(&self, index: isize) -> &E {
        &self.a[index as usize]
    }

    fn get(&self, index: isize) -> E {
        self.at(index).clone()
    }

    fn set(&mut self, index: isize, element: E) {
        self.a[index as usize] = element;
    }

    fn compare(&self, a: &E, b: &E) -> Ordering {
        (self.compare)(a, b)
    }

    /// Sorts the list in the interval `left` to `right` (both inclusive).
    fn do_sort(&mut self, left: isize, right: isize) {
        if (right - left) <= INSERTION_SORT_THRESHOLD {
            self.insertion_sort(left, right);
        } else {
            self.dual_pivot_quicksort(left, right);
        }
    }

    fn insertion_sort(&mut self, left: isize, right: isize) {
        for i in (left + 1)..=right {
            let el = self.get(i);
            let mut j = i;
            while (j > left) && self.compare(self.at(j - 1), &el).is_gt() {
                self.set(j, self.get(j - 1));
                j -= 1;
            }
            self.set(j, el);
        }
    }

    fn dual_pivot_quicksort(&mut self, left: isize, right: isize) {
        debug_assert!(right - left > INSERTION_SORT_THRESHOLD);

        // Compute the two pivots by looking at 5 elements.
        let sixth = (right - left + 1) / 6;
        let index1 = left + sixth;
        let index5 = right - sixth;
        let index3 = (left + right) / 2; // The midpoint.
        let index2 = index3 - sixth;
        let index4 = index3 + sixth;

        let mut el1 = self.get(index1);
        let mut el2 = self.get(index2);
        let mut el3 = self.get(index3);
        let mut el4 = self.get(index4);
        let mut el5 = self.get(index5);

        // Sort the selected 5 elements using a sorting network.
        if self.compare(&el1, &el2).is_gt() {
            std::mem::swap(&mut el1, &mut el2);
        }
        if self.compare(&el4, &el5).is_gt() {
            std::mem::swap(&mut el4, &mut el5);
        }
        if self.compare(&el1, &el3).is_gt() {
            std::mem::swap(&mut el1, &mut el3);
        }
        if self.compare(&el2, &el3).is_gt() {
            std::mem::swap(&mut el2, &mut el3);
        }
        if self.compare(&el1, &el4).is_gt() {
            std::mem::swap(&mut el1, &mut el4);
        }
        if self.compare(&el3, &el4).is_gt() {
            std::mem::swap(&mut el3, &mut el4);
        }
        if self.compare(&el2, &el5).is_gt() {
            std::mem::swap(&mut el2, &mut el5);
        }
        if self.compare(&el2, &el3).is_gt() {
            std::mem::swap(&mut el2, &mut el3);
        }
        if self.compare(&el4, &el5).is_gt() {
            std::mem::swap(&mut el4, &mut el5);
        }

        let pivot1 = el2;
        let pivot2 = el4;

        // el2 and el4 have been saved in the pivot variables. They will be written
        // back, once the partitioning is finished.
        self.set(index1, el1);
        self.set(index3, el3);
        self.set(index5, el5);

        self.set(index2, self.get(left));
        self.set(index4, self.get(right));

        let mut less = left + 1; // First element in the middle partition.
        let mut great = right - 1; // Last element in the middle partition.

        let pivots_are_equal = self.compare(&pivot1, &pivot2).is_eq();
        if pivots_are_equal {
            let pivot = &pivot1;
            // Degenerated case where the partitioning becomes a Dutch national flag
            // problem.
            //
            // [ |  < pivot  | == pivot | unpartitioned | > pivot  | ]
            //  ^             ^          ^             ^            ^
            // left         less         k           great         right
            //
            // a[left] and a[right] are undefined and are filled after the
            // partitioning.
            //
            // Invariants:
            //   1) for x in ]left, less[ : x < pivot.
            //   2) for x in [less, k[ : x == pivot.
            //   3) for x in ]great, right[ : x > pivot.
            let mut k = less;
            while k <= great {
                let ak = self.get(k);
                match self.compare(&ak, pivot) {
                    Ordering::Equal => {}
                    Ordering::Less => {
                        if k != less {
                            self.set(k, self.get(less));
                            self.set(less, ak);
                        }
                        less += 1;
                    }
                    Ordering::Greater => {
                        // Find the first element <= pivot in the range [k - 1, great] and
                        // put [:ak:] there. We know that such an element must exist:
                        // When k == less, then el3 (which is equal to pivot) lies in the
                        // interval. Otherwise a[k - 1] == pivot and the search stops at k-1.
                        // Note that in the latter case invariant 2 will be violated for a
                        // short amount of time. The invariant will be restored when the
                        // pivots are put into their final positions.
                        loop {
                            let comp = self.compare(self.at(great), pivot);
                            if comp.is_gt() {
                                great -= 1;
                                // This is the only location in the while-loop where a new
                                // iteration is started.
                                continue;
                            } else if comp.is_lt() {
                                // Triple exchange.
                                self.set(k, self.get(less));
                                self.set(less, self.get(great));
                                less += 1;
                                self.set(great, ak);
                                great -= 1;
                                break;
                            } else {
                                // comp == 0;
                                self.set(k, self.get(great));
                                self.set(great, ak);
                                great -= 1;
                                // Note: if great < k then we will exit the outer loop and fix
                                // invariant 2 (which we just violated).
                                break;
                            }
                        }
                    }
                }
                k += 1;
            }
        } else {
            // We partition the list into three parts:
            //  1. < pivot1
            //  2. >= pivot1 && <= pivot2
            //  3. > pivot2
            //
            // During the loop we have:
            // [ | < pivot1 | >= pivot1 && <= pivot2 | unpartitioned  | > pivot2  | ]
            //  ^            ^                        ^              ^             ^
            // left         less                     k              great        right
            //
            // a[left] and a[right] are undefined and are filled after the
            // partitioning.
            //
            // Invariants:
            //   1. for x in ]left, less[ : x < pivot1
            //   2. for x in [less, k[ : pivot1 <= x && x <= pivot2
            //   3. for x in ]great, right[ : x > pivot2
            let mut k = less;
            while k <= great {
                let ak = self.get(k);
                let comp_pivot1 = self.compare(&ak, &pivot1);
                if comp_pivot1.is_lt() {
                    if k != less {
                        self.set(k, self.get(less));
                        self.set(less, ak);
                    }
                    less += 1;
                } else {
                    let comp_pivot2 = self.compare(&ak, &pivot2);
                    if comp_pivot2.is_gt() {
                        loop {
                            let comp = self.compare(self.at(great), &pivot2);
                            if comp.is_gt() {
                                great -= 1;
                                if great < k {
                                    break;
                                }
                                // This is the only location inside the loop where a new
                                // iteration is started.
                                continue;
                            } else {
                                // a[great] <= pivot2.
                                let comp = self.compare(self.at(great), &pivot1);
                                if comp.is_lt() {
                                    // Triple exchange.
                                    self.set(k, self.get(less));
                                    self.set(less, self.get(great));
                                    less += 1;
                                    self.set(great, ak);
                                    great -= 1;
                                } else {
                                    // a[great] >= pivot1.
                                    self.set(k, self.get(great));
                                    self.set(great, ak);
                                    great -= 1;
                                }
                                break;
                            }
                        }
                    }
                }
                k += 1;
            }
        }

        // Move pivots into their final positions.
        // We shrunk the list from both sides (a[left] and a[right] have
        // meaningless values in them) and now we move elements from the first
        // and third partition into these locations so that we can store the
        // pivots.
        self.set(left, self.get(less - 1));
        self.set(less - 1, pivot1.clone());
        self.set(right, self.get(great + 1));
        self.set(great + 1, pivot2.clone());

        // The list is now partitioned into three partitions:
        // [ < pivot1   | >= pivot1 && <= pivot2   |  > pivot2   ]
        //  ^            ^                        ^             ^
        // left         less                     great        right

        // Recursive descent. (Don't include the pivot values.)
        self.do_sort(left, less - 2);
        self.do_sort(great + 2, right);

        if pivots_are_equal {
            // All elements in the second partition are equal to the pivot. No
            // need to sort them.
            return;
        }

        // In theory it should be enough to call _doSort recursively on the second
        // partition.
        // The Android source however removes the pivot elements from the recursive
        // call if the second partition is too large (more than 2/3 of the list).
        if less < index1 && great > index5 {
            while self.compare(self.at(less), &pivot1).is_eq() {
                less += 1;
            }
            while self.compare(self.at(great), &pivot2).is_eq() {
                great -= 1;
            }

            // Copy paste of the previous 3-way partitioning with adaptions.
            //
            // We partition the list into three parts:
            //  1. == pivot1
            //  2. > pivot1 && < pivot2
            //  3. == pivot2
            //
            // During the loop we have:
            // [ == pivot1 | > pivot1 && < pivot2 | unpartitioned  | == pivot2 ]
            //              ^                      ^              ^
            //            less                     k              great
            //
            // Invariants:
            //   1. for x in [ *, less[ : x == pivot1
            //   2. for x in [less, k[ : pivot1 < x && x < pivot2
            //   3. for x in ]great, * ] : x == pivot2
            let mut k = less;
            while k <= great {
                let ak = self.get(k);
                let comp_pivot1 = self.compare(&ak, &pivot1);
                if comp_pivot1.is_eq() {
                    if k != less {
                        self.set(k, self.get(less));
                        self.set(less, ak);
                    }
                    less += 1;
                } else {
                    let comp_pivot2 = self.compare(&ak, &pivot2);
                    if comp_pivot2.is_eq() {
                        loop {
                            let comp = self.compare(self.at(great), &pivot2);
                            if comp.is_eq() {
                                great -= 1;
                                if great < k {
                                    break;
                                }
                                // This is the only location inside the loop where a new
                                // iteration is started.
                                continue;
                            } else {
                                // a[great] < pivot2.
                                let comp = self.compare(self.at(great), &pivot1);
                                if comp.is_lt() {
                                    // Triple exchange.
                                    self.set(k, self.get(less));
                                    self.set(less, self.get(great));
                                    less += 1;
                                    self.set(great, ak);
                                    great -= 1;
                                } else {
                                    // a[great] == pivot1.
                                    self.set(k, self.get(great));
                                    self.set(great, ak);
                                    great -= 1;
                                }
                                break;
                            }
                        }
                    }
                }
                k += 1;
            }
            // The second partition has now been cleared of pivot elements and looks
            // as follows:
            // [  *  |  > pivot1 && < pivot2  | * ]
            //        ^                      ^
            //       less                  great
            // Sort the second partition using recursive descent.
            self.do_sort(less, great);
        } else {
            // The second partition looks as follows:
            // [  *  |  >= pivot1 && <= pivot2  | * ]
            //        ^                        ^
            //       less                    great
            // Simply sort it by recursive descent.
            self.do_sort(less, great);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    // The expected values are the Dart VM's (Dart 3.13.1), printed by
    // `final r = Random(0x42688); [for (var i = 0; i < 8; i++) r.nextInt(1000)]`
    // and the same with 256 and 4294967296; 0x42688 is QuantizerWsmeans's seed.

    #[test]
    fn random_with_the_wsmeans_seed_gives_dart_s_sequence() {
        let mut random = Random::new(0x42688);
        let values: Vec<u64> = (0..8).map(|_| random.next_int(1000)).collect();
        assert_eq!(values, [693, 708, 2, 222, 860, 269, 230, 84]);
    }

    #[test]
    fn random_with_a_power_of_two_takes_dart_s_fast_path() {
        let mut random = Random::new(0x42688);
        let values: Vec<u64> = (0..4).map(|_| random.next_int(256)).collect();
        assert_eq!(values, [141, 12, 66, 62]);
    }

    #[test]
    fn random_up_to_two_to_the_thirty_second_gives_dart_s_sequence() {
        let mut random = Random::new(0x42688);
        let values: Vec<u64> = (0..4).map(|_| random.next_int(1 << 32)).collect();
        assert_eq!(values, [552503693, 3289373708, 273096002, 1621756222]);
    }

    #[test]
    fn max_and_min_order_signed_zeros_as_dart_does() {
        assert!(max(-0.0, 0.0).is_sign_positive());
        assert!(max(0.0, -0.0).is_sign_positive());
        assert!(min(-0.0, 0.0).is_sign_negative());
        assert!(min(0.0, -0.0).is_sign_negative());
    }

    #[test]
    fn compare_to_orders_signed_zeros_and_nan_as_dart_does() {
        assert_eq!(compare_to(1.0, 2.0), Ordering::Less);
        assert_eq!(compare_to(2.0, 1.0), Ordering::Greater);
        assert_eq!(compare_to(1.0, 1.0), Ordering::Equal);
        assert_eq!(compare_to(-0.0, 0.0), Ordering::Less);
        assert_eq!(compare_to(0.0, -0.0), Ordering::Greater);
        assert_eq!(compare_to(f64::NAN, 1.0), Ordering::Greater);
        assert_eq!(compare_to(1.0, f64::NAN), Ordering::Less);
        assert_eq!(compare_to(f64::NAN, f64::NAN), Ordering::Equal);
    }

    // The expected values are the Dart VM's: `1e300.round()`, `(-1e300).floor()`,
    // `2.5.round()` and `(-2.5).round()`.
    #[test]
    fn round_floor_and_ceil_give_dart_s_ints() {
        assert_eq!(round(1e300), i64::MAX);
        assert_eq!(floor(-1e300), i64::MIN);
        assert_eq!(round(2.5), 3);
        assert_eq!(round(-2.5), -3);
        assert_eq!(floor(-0.5), -1);
        assert_eq!(ceil(-0.5), 0);
    }

    #[test]
    #[should_panic(expected = "Infinity or NaN toInt")]
    fn round_of_nan_panics_where_dart_throws() {
        round(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "Infinity or NaN toInt")]
    fn floor_of_infinity_panics_where_dart_throws() {
        floor(f64::INFINITY);
    }

    #[test]
    fn max_and_min_propagate_nan_as_dart_does() {
        assert!(max(f64::NAN, 1.0).is_nan());
        assert!(max(1.0, f64::NAN).is_nan());
        assert!(min(f64::NAN, 1.0).is_nan());
        assert!(min(1.0, f64::NAN).is_nan());
        assert_eq!(max(1.0, 2.0), 2.0);
        assert_eq!(min(1.0, 2.0), 1.0);
    }

    // The expected values are the Dart VM's (Dart 3.13.1): `-360.0 % 360.0`, `-0.0 % 360.0`
    // and `(-pi * 16 + pi * 8) % (pi * 2)` are `0.0` with `isNegative` false.
    #[test]
    fn modulo_gives_a_positive_zero_and_a_positive_remainder_as_dart_does() {
        assert!(modulo(-360.0, 360.0).is_sign_positive());
        assert_eq!(modulo(-360.0, 360.0), 0.0);
        assert!(modulo(-0.0, 360.0).is_sign_positive());
        let radians = modulo(-PI * 16.0 + PI * 8.0, PI * 2.0);
        assert!(radians.is_sign_positive());
        assert_eq!(radians, 0.0);
        assert_eq!(modulo(-5.0, 3.0), 1.0);
        assert_eq!(modulo(5.0, -3.0), 2.0);
        assert_eq!(modulo(-5.0, -3.0), 1.0);
        assert_eq!(modulo(370.5, 360.0), 10.5);
    }

    /// The ids of `keys`, sorted by key with [`sort`].
    fn sorted_ids(keys: &[i64]) -> Vec<usize> {
        let mut items: Vec<(i64, usize)> = keys.iter().copied().zip(0..).collect();
        sort(&mut items, |a, b| a.0.cmp(&b.0));
        assert!(items.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        items.into_iter().map(|(_, id)| id).collect()
    }

    // The expected values are the Dart VM's (Dart 3.13.1), printed by sorting the records
    // `(keys[i], i)` with `items.sort((a, b) => a.$1.compareTo(b.$1))` and listing each `$2`.

    #[test]
    fn sort_keeps_ties_in_order_up_to_thirty_three_elements_only() {
        let keys: Vec<i64> = (0..33).map(|i| (i * 7) % 5).collect();
        assert_eq!(
            sorted_ids(&keys),
            [
                0, 5, 10, 15, 20, 25, 30, 3, 8, 13, 18, 23, 28, 1, 6, 11, 16, 21, 26, 31, 4, 9, 14,
                19, 24, 29, 2, 7, 12, 17, 22, 27, 32
            ]
        );
        let keys: Vec<i64> = (0..34).map(|i| (i * 7) % 5).collect();
        assert_eq!(
            sorted_ids(&keys),
            [
                20, 30, 5, 10, 0, 25, 15, 28, 8, 3, 13, 23, 33, 18, 26, 1, 31, 6, 11, 21, 16, 14,
                24, 9, 29, 4, 19, 17, 22, 12, 27, 7, 2, 32
            ]
        );
    }

    #[test]
    fn sort_orders_ties_as_dart_s_dual_pivot_quicksort_does() {
        let keys: Vec<i64> = (0..50).map(|i| (i * 7) % 5).collect();
        assert_eq!(
            sorted_ids(&keys),
            [
                5, 10, 45, 15, 0, 20, 25, 40, 30, 35, 38, 48, 3, 8, 13, 18, 43, 23, 28, 33, 16, 21,
                6, 46, 41, 26, 31, 1, 36, 11, 14, 4, 9, 29, 44, 49, 34, 19, 39, 24, 27, 32, 42, 22,
                17, 12, 7, 47, 2, 37
            ]
        );
    }

    #[test]
    fn sort_with_equal_pivots_orders_ties_as_dart_does() {
        assert_eq!(
            sorted_ids(&[0; 40]),
            [
                13, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 39, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 25
            ]
        );
    }

    #[test]
    fn sort_with_a_large_middle_partition_orders_ties_as_dart_does() {
        let keys: Vec<i64> = (0..100).map(|i| i % 3).collect();
        assert_eq!(
            sorted_ids(&keys),
            [
                30, 3, 6, 9, 12, 15, 33, 18, 21, 24, 27, 96, 0, 36, 39, 42, 45, 48, 51, 54, 57, 93,
                63, 99, 66, 69, 72, 75, 78, 81, 84, 87, 90, 60, 16, 88, 13, 37, 40, 43, 1, 46, 49,
                7, 52, 19, 55, 58, 61, 22, 64, 76, 91, 79, 97, 94, 28, 10, 31, 73, 70, 85, 25, 4,
                82, 67, 34, 23, 68, 62, 8, 71, 26, 59, 74, 20, 98, 77, 35, 53, 80, 29, 50, 83, 11,
                17, 86, 47, 56, 89, 32, 44, 92, 5, 41, 95, 14, 38, 2, 65
            ]
        );
    }

    #[test]
    fn sort_of_an_empty_or_single_list_leaves_it() {
        let mut empty: Vec<i64> = Vec::new();
        sort(&mut empty, i64::cmp);
        assert!(empty.is_empty());
        let mut single = vec![7];
        sort(&mut single, i64::cmp);
        assert_eq!(single, [7]);
    }
}
