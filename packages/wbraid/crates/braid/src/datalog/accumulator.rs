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

/// Why [`AccumulatorSet::add`] refused a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccumulatorError {
    /// The trustee index already holds a different value.
    Conflict { index: TrusteeIndex },
    /// The value is already held at another trustee index.
    Duplicate { index: TrusteeIndex },
    /// The trustee index is `0`, which is not a trustee, or does not fit in
    /// the accumulator.
    OutOfRange { index: TrusteeIndex },
}

impl std::fmt::Display for AccumulatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccumulatorError::Conflict { index } => {
                write!(f, "trustee {} already contributed a different value", index)
            }
            AccumulatorError::Duplicate { index } => write!(
                f,
                "trustee {} contributed a value another trustee already contributed",
                index
            ),
            AccumulatorError::OutOfRange { index } => {
                write!(f, "trustee index {} is out of range", index)
            }
        }
    }
}

impl std::error::Error for AccumulatorError {}

impl<T: Ord + std::fmt::Debug + Clone> AccumulatorSet<T> {
    /// Create an accumulator initialized with the first trustee's value.
    ///
    /// The initial value is stored at trustee index `1`.
    pub fn new(init: T) -> Self {
        let mut values: [Option<T>; ACCUMULATOR_CAPACITY] = std::array::from_fn(|_| None);
        values[1] = Some(init.clone());
        AccumulatorSet {
            values,
            value_set: BTreeSet::from([init]),
        }
    }

    /// Add `rhs` at `index`, or report the uniqueness invariant it violates.
    ///
    /// Idempotent for an identical `(value, index)` pair. Fails if `index`
    /// already holds a *different* value, if `rhs` already appears at another
    /// index, or if `index` is `0` or does not fit in the accumulator. The
    /// values come from board messages, so the datalog rules turn a failure
    /// into an `error` fact, which halts the protocol like the `collides` rule
    /// does.
    pub(crate) fn add(&self, rhs: T, index: TrusteeIndex) -> Result<Self, AccumulatorError> {
        if index == 0 {
            return Err(AccumulatorError::OutOfRange { index });
        }
        let slot = self
            .values
            .get(index)
            .ok_or(AccumulatorError::OutOfRange { index })?;
        match slot {
            Some(existing) if *existing == rhs => return Ok(self.clone()),
            Some(_) => return Err(AccumulatorError::Conflict { index }),
            None if self.value_set.contains(&rhs) => {
                return Err(AccumulatorError::Duplicate { index })
            }
            None => {}
        }

        let mut ret = self.clone();
        ret.value_set.insert(rhs.clone());
        ret.values[index] = Some(rhs);
        Ok(ret)
    }

    /// Extract all present values in trustee-index order.
    pub(crate) fn extract(&self) -> Vec<T> {
        self.values.iter().flatten().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{AccumulatorError, AccumulatorSet, ACCUMULATOR_CAPACITY};

    #[test]
    fn add_accumulates_in_index_order() {
        let acc = AccumulatorSet::new(10)
            .add(20, 2)
            .expect("a fresh value at a fresh index");

        assert_eq!(acc.extract(), vec![10, 20]);
    }

    #[test]
    fn add_is_idempotent_for_the_same_value_at_the_same_index() {
        let acc = AccumulatorSet::new(10);

        assert_eq!(acc.add(10, 1), Ok(acc.clone()));
    }

    #[test]
    fn add_rejects_a_different_value_at_a_taken_index() {
        let acc = AccumulatorSet::new(10);

        assert_eq!(acc.add(20, 1), Err(AccumulatorError::Conflict { index: 1 }));
    }

    #[test]
    fn add_rejects_a_value_already_held_at_another_index() {
        let acc = AccumulatorSet::new(10);

        assert_eq!(
            acc.add(10, 2),
            Err(AccumulatorError::Duplicate { index: 2 })
        );
    }

    #[test]
    fn add_rejects_index_zero() {
        let acc = AccumulatorSet::new(10);

        assert_eq!(
            acc.add(20, 0),
            Err(AccumulatorError::OutOfRange { index: 0 })
        );
        assert_eq!(acc.extract(), vec![10]);
    }

    #[test]
    fn add_rejects_an_index_beyond_the_capacity() {
        let acc = AccumulatorSet::new(10);

        assert_eq!(
            acc.add(20, ACCUMULATOR_CAPACITY),
            Err(AccumulatorError::OutOfRange {
                index: ACCUMULATOR_CAPACITY
            })
        );
    }
}
