// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Counting algorithm error handling.
mod error;
/// Instant runoff voting counting algorithm.
pub mod instant_runoff;
/// Plurality at large counting algorithm.
pub mod plurality_at_large;
/// Utility functions for counting algorithms.
pub mod utils;

/// Tally counting algorithm implementation.
#[allow(clippy::module_inception)]
mod counting_algorithm;
pub use counting_algorithm::*;
