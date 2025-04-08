//! # Interval operations for Range
//!
//! This module exports the `Interval` trait that extends the core
//! `Range` type with useful interval operations.

use std::ops::Range;

/// A simple trait for intervals math.
///
/// We use this to extend [Range] with useful interval functionality.
pub trait Interval: PartialEq {
    /// The underlying numerical type.
    type Element: Copy + Ord;

    /// Return the intersection of two intervals.
    fn intersection(&self, other: &Self) -> Self;

    /// Return true, if `other` is completely contained with the
    /// interval.
    fn contains_interval(&self, other: &Self) -> bool;

    /// Return true, if the two intervals have overlapping parts.
    fn overlaps(&self, other: &Self) -> bool;
}

impl<T: Copy + Ord + Default> Interval for Range<T> {
    type Element = T;

    fn intersection(&self, other: &Self) -> Self {
        self.start.max(other.start)..self.end.min(other.end)
    }

    fn contains_interval(&self, other: &Self) -> bool {
        self.intersection(other) == *other
    }

    fn overlaps(&self, other: &Self) -> bool {
        !self.is_empty() && !self.intersection(other).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    type Ivl = Range<u64>;

    /// The boolean implication operation.
    fn implies(a: bool, b: bool) -> bool {
        !a || b
    }

    fn ivl_equal(a: Ivl, b: Ivl) -> bool {
        (a.is_empty() && b.is_empty()) || (a == b)
    }

    proptest! {
        #[test]
        fn intersection_semantics(v: u64, ivl1: Ivl, ivl2: Ivl) {
            assert_eq!(ivl1.contains(&v) && ivl2.contains(&v),
                       ivl1.intersection(&ivl2).contains(&v));
        }

        #[test]
        fn intersection_with_empty(ivl: Ivl)
        {
            assert!(ivl_equal(ivl.intersection(&Default::default()), Default::default()));
        }

        #[test]
        fn intersection_is_reflexive(ivl: Ivl) {
            assert!(ivl_equal(ivl.intersection(&ivl), ivl));
        }

        #[test]
        fn intersection_is_commutative(ivl1: Ivl, ivl2: Ivl) {
            assert!(ivl_equal(ivl1.intersection(&ivl2), ivl2.intersection(&ivl1)));
        }

        #[test]
        fn intersection_is_associative(ivl1: Ivl, ivl2: Ivl, ivl3: Ivl) {
            assert!(ivl_equal(ivl1.intersection(&ivl2.intersection(&ivl3)),
                              ivl1.intersection(&ivl2).intersection(&ivl3)));
        }

        #[test]
        fn overlaps_semantics(v: u64, ivl1: Ivl, ivl2: Ivl) {
            assert!(implies(ivl1.contains(&v) && ivl2.contains(&v),
                            ivl1.overlaps(&ivl2)));
        }

        #[test]
        fn overlaps_is_reflexive(ivl: Ivl) {
            assert!(ivl.overlaps(&ivl));
        }

        #[test]
        fn overlaps_is_commutative(ivl1: Ivl, ivl2: Ivl) {
            assert_eq!(ivl1.overlaps(&ivl2), ivl2.overlaps(&ivl1));
        }

        #[test]
        fn overlaps_empty(ivl: Ivl) {
            let empty = Ivl::default();

            assert!(!ivl.overlaps(&empty));
            assert!(!empty.overlaps(&ivl));
        }

        #[test]
        fn contains_symmetric_for_identical_values(ivl1: Ivl, ivl2: Ivl) {
            // If two intervals contain each other, they are
            // identical.
            assert!(implies(ivl1.contains_interval(&ivl2) && ivl2.contains_interval(&ivl1),
                            ivl_equal(ivl1, ivl2)));
        }

        #[test]
        fn contains_is_transitive(ivl1: Ivl, ivl2: Ivl, ivl3: Ivl) {
            assert!(implies(ivl1.contains_interval(&ivl2) && ivl2.contains_interval(&ivl3),
                            ivl1.contains_interval(&ivl3)));
        }
    }

    #[test]
    fn interval_intersection() {
        let empty_ivl = Ivl::default();
        let first_ivl = Ivl { start: 10, end: 20 };
        let second_ivl = Ivl { start: 15, end: 25 };
        let unrelated_ivl = Ivl {
            start: 80,
            end: 100,
        };

        let covering_ivl = Ivl { start: 5, end: 70 };

        // Basic sanity checking
        assert!(empty_ivl.is_empty());

        // Functionality
        assert_eq!(
            first_ivl.intersection(&second_ivl),
            Range { start: 15, end: 20 }
        );
        assert_eq!(first_ivl.intersection(&covering_ivl), first_ivl);
        assert!(first_ivl.intersection(&unrelated_ivl).is_empty());

        // Reflexitivy
        assert_eq!(first_ivl.intersection(&first_ivl), first_ivl);

        // Commutativity
        assert_eq!(covering_ivl.intersection(&first_ivl), first_ivl);
    }

    #[test]
    fn interval_contains_interval() {
        let first_ivl = Ivl { start: 10, end: 20 };
        let second_ivl = Ivl { start: 15, end: 25 };
        let contained_ivl = Ivl { start: 11, end: 14 };

        assert!(!first_ivl.contains_interval(&second_ivl));
        assert!(first_ivl.contains_interval(&contained_ivl));
        assert!(!second_ivl.contains_interval(&contained_ivl));
    }
}
