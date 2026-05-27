// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub use super::error::{Error, Result};
use crate::pipes::do_tally::ContestResult;

/// Trait for implementing counting algorithms that perform tally operations.
///
/// Implementations calculate contest results based on ballots and contest configuration.
pub trait CountingAlgorithm {
    /// Performs the tally operation for a contest.
    ///
    /// # Errors
    ///
    /// Returns an error if the tally operation fails.
    fn tally(&self) -> Result<ContestResult>;
}
