// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Error handling for pipe operations.
pub mod error;
/// Pipe input configuration and data structures.
pub mod pipe_inputs;
/// Pipeline stage names and identifiers.
pub mod pipe_name;

// Pipes
/// Ballot image generation from vote records.
pub mod ballot_images;
/// Ballot decoding and vote extraction.
pub mod decode_ballots;
/// Tally computation and vote counting.
pub mod do_tally;
/// Database generation from election results.
pub mod generate_db;
/// Report generation for election results.
pub mod generate_reports;
/// Winner identification and tracking.
pub mod mark_winners;

/// Pipeline processing and routing.
#[allow(clippy::module_inception)]
mod pipes;
pub use pipes::*;
