// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Hash accumulator: a fixed-capacity, trustee-indexed set used as a datalog
//! relation column (§7.7 of `crates/braid/v0.6_spec.md`).
//!
//! During the DKG and decryption phases the engine must accumulate one content
//! hash per trustee, in trustee-index order, while enforcing two invariants:
//! a given trustee index holds at most one value, and no value repeats across
//! indices. Encoding this as an [`AccumulatorSet`] (rather than a bare `Vec`)
//! lets the ascent rules fold hashes in monotonically — `shares_acc(.., n)` is
//! built from `shares_acc(.., n-1)` plus trustee `n`'s hash — and lets
//! [`AccumulatorSet::extract`] read them back out in index order for the
//! action layer.
//!
//! Ported from the vs_lift `ascent_logic::utils` module.

use crate::messages::newtypes::TrusteeIndex;
use std::collections::BTreeSet;

/// Capacity of the backing array. Trustee indices are 1-based (§4.3), so the
/// array must hold `MAX_TRUSTEES + 1` slots (slot `0` is unused). This ties the
/// accumulator to the system-wide trustee limit rather than an independent
/// constant.
const ACCUMULATOR_CAPACITY: usize = crate::messages::newtypes::MAX_TRUSTEES + 1;

/// Fixed-capacity set keyed by trustee index, enforcing uniqueness invariants.
///
/// `values[i]` holds the value contributed by trustee `i` (1-based);
/// `value_set` mirrors the present values for O(log n) duplicate detection.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct AccumulatorSet<T> {
    values: [Option<T>; ACCUMULATOR_CAPACITY],
    value_set: BTreeSet<T>,
}

impl<T: Ord + std::fmt::Debug + Clone> AccumulatorSet<T> {
    /// Create an accumulator initialized with the first trustee's value.
    ///
    /// The initial value is stored at trustee index `1`.
    pub fn new(init: T) -> Self {
        AccumulatorSet {
            values: std::array::from_fn(|_| None),
            value_set: BTreeSet::new(),
        }
        .add(init, 1)
    }

    /// Add `rhs` at `index`, panicking if it violates a uniqueness invariant.
    ///
    /// Idempotent for an identical `(value, index)` pair; panics if `index`
    /// already holds a *different* value, or if `rhs` already appears at another
    /// index. The panic mirrors the datalog `collides` halt: a well-formed,
    /// non-equivocating input set can never trigger it.
    pub(crate) fn add(&self, rhs: T, index: TrusteeIndex) -> Self {
        let existing = self.values[index].clone();
        // If the slot at `index` is already set, it must match the supplied value.
        if let Some(existing) = existing {
            if existing != rhs {
                panic!(
                    "Attempted to add different value at index {}: existing {:?}, new {:?}",
                    index, existing, rhs
                );
            } else {
                // Value already present at this index: no change needed.
                return self.clone();
            }
        }
        // If the slot is empty, `rhs` must not already appear at a different index.
        else if self.value_set.contains(&rhs) {
            panic!(
                "Attempted to add duplicate value {:?} at index {}",
                rhs, index
            );
        }

        // The addition is valid.
        let mut ret = AccumulatorSet {
            values: self.values.clone(),
            value_set: self.value_set.clone(),
        };
        ret.value_set.insert(rhs.clone());
        ret.values[index] = Some(rhs.clone());

        ret
    }

    /// Extract all present values in trustee-index order.
    pub(crate) fn extract(&self) -> Vec<T> {
        self.values.iter().flatten().cloned().collect()
    }
}
