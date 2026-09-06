// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod error;
pub mod pipe_inputs;
pub mod pipe_name;

// Pipes
pub mod ballot_images;
pub mod decode_ballots;
pub mod do_tally;
pub mod generate_db;
pub mod generate_reports;
pub mod mark_winners;

#[expect(
    clippy::module_inception,
    reason = "Preserve the existing module layout, re-exports and caller paths during the construct review"
)]
mod pipes;
pub use pipes::*;
