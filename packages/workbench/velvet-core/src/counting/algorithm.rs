// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Counting-algorithm trait.

use rand_core::RngCore;

use crate::result::ContestResult;

use super::error::Result;

/// Common interface implemented by every plaintext counting algorithm
/// (plurality-at-large, instant-runoff, etc.). Implementations are
/// expected to be pure functions over already-decoded ballots — no I/O,
/// no async, no global state.
///
/// The `rng` parameter is threaded through so randomness used for
/// tie-breaking (e.g. IRV's `determine_winner_by_lot`) is injectable
/// and testable. Algorithms that don't need randomness ignore it.
pub trait CountingAlgorithm {
    fn tally(&self, rng: &mut dyn RngCore) -> Result<ContestResult>;
}
