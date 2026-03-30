// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Counting algorithms for tally operations.
pub mod counting_algorithm;
/// Tally operation error types.
mod error;
/// Tally operations and result aggregation.
pub mod tally;

/// Tally processing and result aggregation.
#[allow(clippy::module_inception)]
mod do_tally;

pub use do_tally::*;
