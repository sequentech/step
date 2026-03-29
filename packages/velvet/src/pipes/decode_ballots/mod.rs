// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot decoding pipes for processing encoded ballots.
#[allow(clippy::module_inception)]
/// Single ballot decoding operations.
mod decode_ballots;
/// Multi-ballot decoding operations.
pub(crate) mod decode_mcballots;
pub use decode_ballots::*;
